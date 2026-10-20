//! OpenRouter Multi-Model AI Language Immersion Engine
//!
//! Provides intelligent contextual explanation, learning dictionary generation,
//! and sentence structure breakdowns with automatic multi-model failover
//! and output sanitization.
//!
//! Latency notes (measured against the live free tier):
//! - The old chain started with `liquid/lfm-2.5-2.6b:free`, whose provider answered
//!   `504` after ~25 s. Its replacement starts with `dots-studio/dots-3-note-preview`,
//!   which streamed its first token in ~0.8 s on a dictionary-sized prompt.
//! - Several free models are reasoning models. With `reasoning.effort = "low"` they can
//!   spend the whole `max_tokens` budget on hidden reasoning and return no content at all.
//!   The chain therefore sets `effort = "none"` where the provider allows it and never
//!   falls back to the `reasoning` field as if it were the answer.
//! - Each tab used to fire its own request immediately. With a free tier that queues per
//!   model, four calls compete with the one the user is looking at. The popup now asks for
//!   the visible tab first, then prefetches the others one at a time.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tauri::ipc::Channel;

use crate::db::Db;
use crate::state::AppState;

/// One curated free model plus the reasoning effort that actually makes it answer.
#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub id: &'static str,
    pub effort: &'static str,
}

/// Order matters: the first usable model wins. `dots` is the fastest observed and is the
/// only one that needs `effort = "none"` (with `low` it burns the budget on reasoning).
pub const FREE_MODEL_CHAIN: &[ModelSpec] = &[
    ModelSpec { id: "dots-studio/dots-3-note-preview:free", effort: "none" },
    ModelSpec { id: "poolside/laguna-s-2.1:free", effort: "none" },
    ModelSpec { id: "minimax/minimax-m3.1-flash:free", effort: "low" },
    ModelSpec { id: "google/gemma-4-26b-a4b-it:free", effort: "low" },
    ModelSpec { id: "google/gemma-4-31b-it:free", effort: "low" },
];

const NON_STREAM_TIMEOUT: Duration = Duration::from_secs(10);
const STREAM_HEADER_TIMEOUT: Duration = Duration::from_secs(8);
const STREAM_FIRST_CONTENT_TIMEOUT: Duration = Duration::from_secs(9);
const STREAM_CHUNK_TIMEOUT: Duration = Duration::from_secs(12);
const TOTAL_DEADLINE: Duration = Duration::from_secs(35);
const SHORT_COOLDOWN: Duration = Duration::from_secs(45);
const INVALID_COOLDOWN: Duration = Duration::from_secs(30);
const GONE_COOLDOWN: Duration = Duration::from_secs(600);
const DYNAMIC_MODELS_TTL: Duration = Duration::from_secs(600);
const DYNAMIC_MODELS_LIMIT: usize = 4;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static STREAM_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static DYNAMIC_MODELS_CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>)>>> = OnceLock::new();
static MODEL_COOLDOWNS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(4))
        .pool_max_idle_per_host(8)
        .tcp_keepalive(Duration::from_secs(60))
}

fn get_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        client_builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default()
    })
}

/// Streaming requests must not inherit a total-request timeout: a response can be useful
/// for a minute while tokens keep arriving. Idle time is bounded per chunk instead.
fn get_stream_client() -> &'static reqwest::Client {
    STREAM_CLIENT.get_or_init(|| client_builder().build().unwrap_or_default())
}

fn cooldowns() -> &'static Mutex<HashMap<String, Instant>> {
    MODEL_COOLDOWNS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A failed model is skipped for a while, so four tabs and repeated popups do not all pay
/// the same timeout before reaching a healthy fallback.
fn model_is_ready(id: &str) -> bool {
    let now = Instant::now();
    let mut map = cooldowns().lock().unwrap();
    if let Some(&until) = map.get(id) {
        if until > now {
            return false;
        }
        map.remove(id);
    }
    true
}

fn mark_model_down(id: &str, duration: Duration) {
    cooldowns().lock().unwrap().insert(id.to_string(), Instant::now() + duration);
}

fn mark_model_up(id: &str) {
    cooldowns().lock().unwrap().remove(id);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiExplainRequest {
    pub word: String,
    pub sentence: String,
    pub category: String,
    pub system_prompt: String,
    pub user_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiExplainResponse {
    pub content: String,
    pub model_used: String,
    pub from_cache: bool,
}

/// Events for the popup's streaming path. `Reset` is sent when a model streamed some
/// content and then failed, so the next model does not append to a half-written answer.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiStreamEvent {
    Start { model: String, cached: bool },
    Delta { content: String },
    Reset { model: String },
    Done { content: String, model: String, cached: bool },
}

#[derive(Debug, Serialize)]
struct OpenRouterMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct OpenRouterReasoning<'a> {
    effort: &'a str,
    exclude: bool,
}

#[derive(Debug, Serialize)]
struct OpenRouterRequestBody<'a> {
    model: &'a str,
    messages: Vec<OpenRouterMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<OpenRouterReasoning<'a>>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: Option<OpenRouterChoiceMessage>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoiceMessage {
    content: Option<String>,
    reasoning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    model: Option<String>,
    choices: Option<Vec<OpenRouterChoice>>,
    #[allow(dead_code)]
    error: Option<OpenRouterErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamResponse {
    model: Option<String>,
    choices: Option<Vec<OpenRouterStreamChoice>>,
    error: Option<OpenRouterErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamChoice {
    delta: Option<OpenRouterStreamDelta>,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamDelta {
    content: Option<String>,
}

#[derive(Debug, Default)]
struct StreamPayload {
    content: String,
    model: Option<String>,
    finish_reason: Option<String>,
    done: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsList {
    data: Option<Vec<OpenRouterModelInfo>>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelInfo {
    id: String,
    pricing: Option<OpenRouterPricing>,
    architecture: Option<OpenRouterArchitecture>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterArchitecture {
    output_modalities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenRouterErrorDetail {
    message: Option<String>,
    code: Option<serde_json::Value>,
}

/// Fetch active free text models from OpenRouter. The list is cached for ten minutes:
/// it used to be fetched only after the whole curated chain failed, adding a round trip
/// to every slow request.
pub async fn fetch_dynamic_free_models() -> Vec<String> {
    let cached = {
        let guard = DYNAMIC_MODELS_CACHE.get_or_init(|| Mutex::new(None));
        let guard = guard.lock().unwrap();
        guard.as_ref().and_then(|(at, models)| {
            (at.elapsed() < DYNAMIC_MODELS_TTL).then(|| models.clone())
        })
    };
    if let Some(models) = cached {
        return models;
    }

    let stale = {
        let guard = DYNAMIC_MODELS_CACHE.get_or_init(|| Mutex::new(None));
        guard.lock().unwrap().as_ref().map(|(_, models)| models.clone())
    };

    let client = get_client();
    let resp = match client
        .get("https://openrouter.ai/api/v1/models")
        .timeout(Duration::from_secs(8))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return stale.unwrap_or_default(),
    };

    if !resp.status().is_success() {
        return stale.unwrap_or_default();
    }

    let data: OpenRouterModelsList = match resp.json().await {
        Ok(d) => d,
        Err(_) => return stale.unwrap_or_default(),
    };

    let mut models = Vec::new();
    if let Some(items) = data.data {
        for item in items {
            let id_lower = item.id.to_lowercase();
            // Skip safety classifiers, moderation tools, embeddings, and routers. The
            // router (`openrouter/free`) hides which model actually answered and can pick
            // a reasoning-only model that never emits content.
            if id_lower.contains("safety")
                || id_lower.contains("guard")
                || id_lower.contains("moderation")
                || id_lower.contains("embed")
                || id_lower.contains("rerank")
                || id_lower == "openrouter/free"
            {
                continue;
            }

            // Lyrics/audio models are also priced at zero, but they cannot answer a chat
            // prompt. Require text output (and reject text+audio hybrids such as Lyria).
            if let Some(arch) = &item.architecture {
                if let Some(out) = &arch.output_modalities {
                    if !out.iter().any(|m| m == "text") || out.iter().any(|m| m != "text") {
                        continue;
                    }
                }
            }

            let is_free_id = item.id.ends_with(":free") && item.id != "openrouter/free";
            let is_free_price = item.pricing.as_ref().is_some_and(|p| {
                is_zero_price(p.prompt.as_deref()) && is_zero_price(p.completion.as_deref())
            });
            if is_free_id || is_free_price {
                models.push(item.id);
            }
        }
    }
    models.truncate(DYNAMIC_MODELS_LIMIT);

    if !models.is_empty() {
        let guard = DYNAMIC_MODELS_CACHE.get_or_init(|| Mutex::new(None));
        *guard.lock().unwrap() = Some((Instant::now(), models.clone()));
    }
    models
}

fn is_zero_price(value: Option<&str>) -> bool {
    value.and_then(|v| v.parse::<f64>().ok()).is_some_and(|v| v == 0.0)
}

/// Validates whether the AI response contains actual immersion learning content
/// and is not a moderation/safety stub (e.g. "User Safety: safe") or truncated response.
pub fn is_valid_immersion_output(content: &str) -> bool {
    let trimmed = content.trim();
    let lower = trimmed.to_lowercase();

    // Must have substantive content (at least 40 characters)
    if trimmed.len() < 40 {
        return false;
    }

    // Reject safety classifier outputs and moderation echoes
    if trimmed.starts_with("User Safety:")
        || trimmed.starts_with("Safety:")
        || lower.contains("user safety: safe")
        || lower.contains("user safety: unsafe")
        || lower.contains("content safety")
        || lower == "safe"
        || lower == "unsafe"
    {
        return false;
    }

    true
}

/// Cleans AI output to remove reasoning traces, thinking blocks, and conversational filler.
pub fn clean_ai_output(raw: &str) -> String {
    let mut text = raw.to_string();

    // 1. Remove <think>...</think> blocks (DeepSeek R1 / Gemini Thinking)
    while let Some(start) = text.find("<think>") {
        if let Some(end) = text.find("</think>") {
            text.replace_range(start..end + 8, "");
        } else {
            // Unclosed think tag: remove up to end
            text.truncate(start);
            break;
        }
    }

    // 2. Remove thinking process header blocks
    if let Some(pos) = text.find("Thinking Process:") {
        if let Some(end_nl) = text[pos..].find("\n\n") {
            text.replace_range(pos..pos + end_nl + 2, "");
        }
    }

    let mut trimmed = text.trim();

    // 3. Remove common conversational introductory greetings
    let conversational_intros = [
        "Sure! Here is the explanation:",
        "Sure! Here is the breakdown:",
        "Certainly! Here is the explanation:",
        "Certainly, here is the breakdown:",
        "Here is the learning dictionary entry:",
        "Here is the explanation:",
        "Here is the breakdown:",
        "Here is the translation:",
        "Tentu, berikut penjelasannya:",
        "Berikut adalah penjelasannya:",
        "Berikut adalah bedah kalimatnya:",
    ];

    for intro in conversational_intros {
        if let Some(stripped) = trimmed.strip_prefix(intro) {
            trimmed = stripped.trim();
        }
    }

    trimmed.to_string()
}

/// Computes a cache key from category, word, and sentence.
fn compute_cache_key(category: &str, word: &str, sentence: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    sentence.trim().hash(&mut hasher);
    let sent_hash = hasher.finish();

    format!("{}:{}:{:x}", category.trim(), word.trim().to_lowercase(), sent_hash)
}

/// Output budgets are per tab: the old flat 2500 let reasoning models ramble, while short
/// tabs only need a few sentences. A dictionary entry is the only large one.
fn max_tokens_for(category: &str) -> u32 {
    match category {
        "learning_dictionary" => 1000,
        "sentence_breakdown" => 700,
        "contextual_translation" => 550,
        "word_explanation" => 400,
        "sentence_translation" => 300,
        "word_translation" => 80,
        _ => 700,
    }
}

#[derive(Debug, Clone)]
struct Attempt {
    id: String,
    /// `None` for dynamically discovered models: their reasoning support is unknown, so
    /// the parameter is omitted rather than risking a 400.
    effort: Option<&'static str>,
}

fn curated_attempts() -> Vec<Attempt> {
    FREE_MODEL_CHAIN
        .iter()
        .map(|spec| Attempt { id: spec.id.to_string(), effort: Some(spec.effort) })
        .collect()
}

fn append_dynamic_attempts(attempts: &mut Vec<Attempt>, dynamic: Vec<String>) {
    for id in dynamic.into_iter().take(DYNAMIC_MODELS_LIMIT) {
        if !attempts.iter().any(|a| a.id == id) {
            attempts.push(Attempt { id, effort: None });
        }
    }
}

/// One-shot (non-streaming) path, used by the settings token test and as an API fallback.
pub async fn query_ai_immersion(
    db: &Db,
    req: AiExplainRequest,
) -> Result<AiExplainResponse, String> {
    let token = db
        .get_setting("openrouter_token")
        .unwrap_or_default()
        .trim()
        .to_string();

    if token.is_empty() {
        return Err("Token OpenRouter belum diatur. Silakan masukkan API Key OpenRouter di menu Pengaturan.".to_string());
    }

    let cache_key = compute_cache_key(&req.category, &req.word, &req.sentence);

    // 1. Check SQLite Persistent Cache first (< 1ms)
    if let Some((content, model_used)) = db.get_ai_cache(&cache_key) {
        if is_valid_immersion_output(&content) {
            return Ok(AiExplainResponse { content, model_used, from_cache: true });
        }
    }

    let client = get_client();
    let deadline = Instant::now() + TOTAL_DEADLINE;
    let mut attempts = curated_attempts();
    let curated_len = attempts.len();
    let mut dynamic_loaded = false;
    let mut last_error = String::new();
    let mut i = 0;

    while i < attempts.len() {
        if Instant::now() >= deadline {
            last_error = "Batas waktu permintaan AI tercapai.".to_string();
            break;
        }

        let id = attempts[i].id.clone();
        let effort = attempts[i].effort;
        i += 1;

        if !model_is_ready(&id) {
            continue;
        }

        let body = OpenRouterRequestBody {
            model: &id,
            messages: vec![
                OpenRouterMessage { role: "system", content: &req.system_prompt },
                OpenRouterMessage { role: "user", content: &req.user_prompt },
            ],
            temperature: 0.2,
            max_tokens: max_tokens_for(&req.category),
            stream: None,
            reasoning: effort.map(|effort| OpenRouterReasoning { effort, exclude: true }),
        };

        let started = Instant::now();
        let response_res = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", token))
            .header("HTTP-Referer", "https://github.com/lingua-miner/limusic")
            .header("X-Title", "Limusic Immersion")
            .header("Content-Type", "application/json")
            .timeout(NON_STREAM_TIMEOUT)
            .json(&body)
            .send()
            .await;

        match response_res {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<OpenRouterResponse>().await {
                        Ok(data) => {
                            let actual_model = data.model.unwrap_or_else(|| id.clone());
                            let message = data
                                .choices
                                .and_then(|choices| choices.into_iter().next())
                                .and_then(|choice| choice.message);
                            let content = message
                                .as_ref()
                                .and_then(|m| m.content.clone())
                                .filter(|c| !c.trim().is_empty());
                            let reasoning_len = message
                                .as_ref()
                                .and_then(|m| m.reasoning.as_ref())
                                .map(|r| r.len())
                                .unwrap_or(0);

                            if let Some(content) = content {
                                let cleaned = clean_ai_output(&content);
                                if is_valid_immersion_output(&cleaned) {
                                    let _ = db.save_ai_cache(
                                        &cache_key,
                                        &req.word,
                                        &req.sentence,
                                        &req.category,
                                        &actual_model,
                                        &cleaned,
                                    );
                                    mark_model_up(&id);
                                    tracing::debug!(
                                        "AI model {} answered in {:?} ({} chars)",
                                        actual_model,
                                        started.elapsed(),
                                        cleaned.len()
                                    );
                                    return Ok(AiExplainResponse {
                                        content: cleaned,
                                        model_used: actual_model,
                                        from_cache: false,
                                    });
                                }
                            }

                            tracing::warn!(
                                "Model {} returned no usable content (reasoning {} chars) in {:?}, trying next fallback...",
                                id,
                                reasoning_len,
                                started.elapsed()
                            );
                            mark_model_down(&id, INVALID_COOLDOWN);
                            last_error = format!(
                                "Model {} tidak menghasilkan jawaban yang bisa dipakai",
                                id
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Model {} returned invalid JSON in {:?}: {}, trying next fallback...",
                                id,
                                started.elapsed(),
                                e
                            );
                            last_error = format!("Respons {} tidak valid: {}", id, e);
                            mark_model_down(&id, INVALID_COOLDOWN);
                        }
                    }
                } else if status.as_u16() == 401 {
                    return Err("Token OpenRouter tidak valid (401 Unauthorized). Periksa kembali API Key Anda di Pengaturan.".to_string());
                } else {
                    let err_text = resp.text().await.unwrap_or_default();
                    let err_short: String = err_text.chars().take(240).collect();
                    tracing::warn!(
                        "OpenRouter model {} returned status {} in {:?}: {}, trying next fallback...",
                        id,
                        status,
                        started.elapsed(),
                        err_short
                    );
                    let cooldown = if status.as_u16() == 404 { GONE_COOLDOWN } else { SHORT_COOLDOWN };
                    mark_model_down(&id, cooldown);
                    last_error = format!("Model {} gagal (status {}): {}", id, status, err_short);
                }
            }
            Err(e) => {
                tracing::warn!(
                    "OpenRouter request error on model {} after {:?}: {}, trying next fallback...",
                    id,
                    started.elapsed(),
                    e
                );
                mark_model_down(&id, SHORT_COOLDOWN);
                last_error = format!("Koneksi ke {} gagal: {}", id, e);
            }
        }

        // Only pay for model discovery after every curated model has actually been tried.
        if !dynamic_loaded && i >= curated_len {
            dynamic_loaded = true;
            let dynamic = fetch_dynamic_free_models().await;
            append_dynamic_attempts(&mut attempts, dynamic);
        }
    }

    Err(format!(
        "Semua model AI OpenRouter sedang sibuk atau tidak dapat dijangkau. Rincian terakhir: {}",
        last_error
    ))
}

/// Streaming command used by the word popup. Events carry the first token to the UI while
/// the rest of the answer is still being generated.
#[tauri::command]
pub async fn ai_explain_word_stream(
    state: tauri::State<'_, Arc<AppState>>,
    word: String,
    sentence: String,
    category: String,
    system_prompt: String,
    user_prompt: String,
    on_event: Channel<AiStreamEvent>,
) -> Result<(), String> {
    let req = AiExplainRequest { word, sentence, category, system_prompt, user_prompt };
    query_ai_immersion_stream(state.db.as_ref(), req, on_event).await
}

pub async fn query_ai_immersion_stream(
    db: &Db,
    req: AiExplainRequest,
    on_event: Channel<AiStreamEvent>,
) -> Result<(), String> {
    let token = db
        .get_setting("openrouter_token")
        .unwrap_or_default()
        .trim()
        .to_string();

    if token.is_empty() {
        return Err("Token OpenRouter belum diatur. Silakan masukkan API Key OpenRouter di menu Pengaturan.".to_string());
    }

    let cache_key = compute_cache_key(&req.category, &req.word, &req.sentence);

    // Cached answers are still sent through the channel, so the popup has one code path.
    if let Some((content, model_used)) = db.get_ai_cache(&cache_key) {
        if is_valid_immersion_output(&content) {
            let _ = on_event.send(AiStreamEvent::Start {
                model: model_used.clone(),
                cached: true,
            });
            let _ = on_event.send(AiStreamEvent::Delta { content: content.clone() });
            let _ = on_event.send(AiStreamEvent::Done {
                content,
                model: model_used,
                cached: true,
            });
            return Ok(());
        }
    }

    let client = get_stream_client();
    let deadline = Instant::now() + TOTAL_DEADLINE;
    let mut attempts = curated_attempts();
    let curated_len = attempts.len();
    let mut dynamic_loaded = false;
    let mut last_error = String::new();
    let mut i = 0;

    while i < attempts.len() {
        if Instant::now() >= deadline {
            last_error = "Batas waktu permintaan AI tercapai.".to_string();
            break;
        }

        let id = attempts[i].id.clone();
        let effort = attempts[i].effort;
        i += 1;

        if !model_is_ready(&id) {
            continue;
        }

        let body = OpenRouterRequestBody {
            model: &id,
            messages: vec![
                OpenRouterMessage { role: "system", content: &req.system_prompt },
                OpenRouterMessage { role: "user", content: &req.user_prompt },
            ],
            temperature: 0.2,
            max_tokens: max_tokens_for(&req.category),
            stream: Some(true),
            reasoning: effort.map(|effort| OpenRouterReasoning { effort, exclude: true }),
        };

        let started = Instant::now();
        let request = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", token))
            .header("HTTP-Referer", "https://github.com/lingua-miner/limusic")
            .header("X-Title", "Limusic Immersion")
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send();

        let response = match tokio::time::timeout(STREAM_HEADER_TIMEOUT, request).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                tracing::warn!(
                    "OpenRouter stream error on model {} after {:?}: {}, trying next fallback...",
                    id,
                    started.elapsed(),
                    e
                );
                mark_model_down(&id, SHORT_COOLDOWN);
                last_error = format!("Koneksi ke {} gagal: {}", id, e);
                continue;
            }
            Err(_) => {
                tracing::warn!(
                    "OpenRouter stream header timeout on model {} after {:?}, trying next fallback...",
                    id,
                    started.elapsed()
                );
                mark_model_down(&id, SHORT_COOLDOWN);
                last_error = format!("Model {} tidak merespon dalam waktu yang wajar", id);
                continue;
            }
        };

        let status = response.status();
        if status.as_u16() == 401 {
            return Err("Token OpenRouter tidak valid (401 Unauthorized). Periksa kembali API Key Anda di Pengaturan.".to_string());
        }
        if !status.is_success() {
            let err_text = response.text().await.unwrap_or_default();
            let err_short: String = err_text.chars().take(240).collect();
            tracing::warn!(
                "OpenRouter stream model {} returned status {} in {:?}: {}, trying next fallback...",
                id,
                status,
                started.elapsed(),
                err_short
            );
            let cooldown = if status.as_u16() == 404 { GONE_COOLDOWN } else { SHORT_COOLDOWN };
            mark_model_down(&id, cooldown);
            last_error = format!("Model {} gagal (status {}): {}", id, status, err_short);
            continue;
        }

        match read_openrouter_stream(response, &id, &on_event).await {
            Ok(outcome) => {
                let cleaned = clean_ai_output(&outcome.content);
                if is_valid_immersion_output(&cleaned) {
                    let _ = db.save_ai_cache(
                        &cache_key,
                        &req.word,
                        &req.sentence,
                        &req.category,
                        &outcome.model,
                        &cleaned,
                    );
                    mark_model_up(&id);
                    tracing::debug!(
                        "AI stream model {} answered in {:?} ({} chars)",
                        outcome.model,
                        started.elapsed(),
                        cleaned.len()
                    );
                    let _ = on_event.send(AiStreamEvent::Done {
                        content: cleaned,
                        model: outcome.model,
                        cached: false,
                    });
                    return Ok(());
                }

                if outcome.emitted {
                    let _ = on_event.send(AiStreamEvent::Reset { model: outcome.model.clone() });
                }
                tracing::warn!(
                    "Model {} streamed no usable content in {:?}, trying next fallback...",
                    id,
                    started.elapsed()
                );
                mark_model_down(&id, INVALID_COOLDOWN);
                last_error = format!("Model {} tidak menghasilkan jawaban yang bisa dipakai", id);
            }
            Err(e) => {
                tracing::warn!(
                    "Model {} stream failed after {:?}: {}, trying next fallback...",
                    id,
                    started.elapsed(),
                    e
                );
                mark_model_down(&id, INVALID_COOLDOWN);
                last_error = format!("Model {} terputus: {}", id, e);
            }
        }

        if !dynamic_loaded && i >= curated_len {
            dynamic_loaded = true;
            let dynamic = fetch_dynamic_free_models().await;
            append_dynamic_attempts(&mut attempts, dynamic);
        }
    }

    Err(format!(
        "Semua model AI OpenRouter sedang sibuk atau tidak dapat dijangkau. Rincian terakhir: {}",
        last_error
    ))
}

#[derive(Debug)]
struct StreamOutcome {
    content: String,
    model: String,
    emitted: bool,
}

/// Read an OpenRouter SSE body. Reasoning deltas are deliberately ignored: `reasoning` is
/// chain-of-thought, not the answer, and using it as fallback content produced nonsense.
async fn read_openrouter_stream(
    response: reqwest::Response,
    requested_model: &str,
    on_event: &Channel<AiStreamEvent>,
) -> Result<StreamOutcome, String> {
    let mut stream = response.bytes_stream();
    let mut pending: Vec<u8> = Vec::new();
    let mut content = String::new();
    let mut actual_model = requested_model.to_string();
    let mut emitted = false;
    let first_content_deadline = Instant::now() + STREAM_FIRST_CONTENT_TIMEOUT;

    loop {
        // A reasoning-only model can keep sending hidden reasoning deltas forever while
        // producing no answer. Abort it at the first-content deadline instead of waiting
        // for max_tokens to run out.
        let now = Instant::now();
        if !emitted && now >= first_content_deadline {
            return Err("model did not start answering in time".to_string());
        }
        let wait = if emitted {
            STREAM_CHUNK_TIMEOUT
        } else {
            (first_content_deadline - now).min(STREAM_CHUNK_TIMEOUT)
        };

        let next = tokio::time::timeout(wait, stream.next()).await;
        let chunk = match next {
            Ok(Some(Ok(chunk))) => chunk,
            Ok(Some(Err(e))) => {
                if emitted {
                    let _ = on_event.send(AiStreamEvent::Reset { model: actual_model.clone() });
                }
                return Err(format!("stream error: {}", e));
            }
            Ok(None) => break,
            Err(_) => {
                if emitted {
                    let _ = on_event.send(AiStreamEvent::Reset { model: actual_model.clone() });
                }
                return Err("stream idle timeout".to_string());
            }
        };

        pending.extend_from_slice(&chunk);
        while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = pending.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            match handle_stream_line(
                line.trim_end_matches('\r'),
                &mut actual_model,
                &mut content,
                &mut emitted,
                on_event,
            )? {
                true => {
                    return Ok(StreamOutcome { content, model: actual_model, emitted });
                }
                false => {}
            }
        }
    }

    // A provider may close without a trailing newline after the final data frame.
    if !pending.is_empty() {
        let line = String::from_utf8_lossy(&pending);
        if handle_stream_line(
            line.trim_end_matches('\r'),
            &mut actual_model,
            &mut content,
            &mut emitted,
            on_event,
        )? {
            return Ok(StreamOutcome { content, model: actual_model, emitted });
        }
    }

    Ok(StreamOutcome { content, model: actual_model, emitted })
}

/// Returns `Ok(true)` when the provider sent `[DONE]`.
fn handle_stream_line(
    line: &str,
    actual_model: &mut String,
    content: &mut String,
    emitted: &mut bool,
    on_event: &Channel<AiStreamEvent>,
) -> Result<bool, String> {
    let Some(payload) = parse_stream_line(line) else {
        return Ok(false);
    };

    if let Some(model) = payload.model {
        *actual_model = model;
    }
    if let Some(error) = payload.error {
        if *emitted {
            let _ = on_event.send(AiStreamEvent::Reset { model: actual_model.clone() });
        }
        return Err(error);
    }
    if payload.done {
        return Ok(true);
    }
    if !payload.content.is_empty() {
        if !*emitted {
            let _ = on_event.send(AiStreamEvent::Start {
                model: actual_model.clone(),
                cached: false,
            });
            *emitted = true;
        }
        content.push_str(&payload.content);
        let _ = on_event.send(AiStreamEvent::Delta { content: payload.content });
    }
    let _ = payload.finish_reason;
    Ok(false)
}

/// Parse one SSE line. Non-`data:` lines (including OpenRouter's `: PROCESSING`
/// keepalives) are ignored.
fn parse_stream_line(line: &str) -> Option<StreamPayload> {
    let data = line.trim().strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(StreamPayload { done: true, ..Default::default() });
    }
    let parsed: OpenRouterStreamResponse = serde_json::from_str(data).ok()?;

    if let Some(error) = parsed.error {
        return Some(StreamPayload {
            error: Some(error.message.unwrap_or_else(|| "OpenRouter stream error".to_string())),
            ..Default::default()
        });
    }

    let choice = parsed.choices?.into_iter().next()?;
    Some(StreamPayload {
        content: choice.delta.and_then(|d| d.content).unwrap_or_default(),
        model: parsed.model,
        finish_reason: choice.finish_reason,
        done: false,
        error: None,
    })
}

/// Test if an OpenRouter token is valid by making a lightweight model ping.
pub async fn test_openrouter_connection(token: &str) -> Result<String, String> {
    let clean_token = token.trim();
    if clean_token.is_empty() {
        return Err("Token tidak boleh kosong.".to_string());
    }

    let client = get_client();
    let mut last_error = String::new();

    for spec in FREE_MODEL_CHAIN {
        let body = OpenRouterRequestBody {
            model: spec.id,
            messages: vec![OpenRouterMessage { role: "user", content: "Say 'OK' only." }],
            temperature: 0.1,
            max_tokens: 10,
            stream: None,
            reasoning: Some(OpenRouterReasoning { effort: spec.effort, exclude: true }),
        };

        let resp_res = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", clean_token))
            .header("HTTP-Referer", "https://github.com/lingua-miner/limusic")
            .header("X-Title", "Limusic Immersion")
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(8))
            .json(&body)
            .send()
            .await;

        match resp_res {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    let resp_body: Result<OpenRouterResponse, _> = resp.json().await;
                    let actual_model = resp_body
                        .ok()
                        .and_then(|d| d.model)
                        .unwrap_or_else(|| spec.id.to_string());
                    return Ok(actual_model);
                } else if status.as_u16() == 401 {
                    return Err("Token OpenRouter tidak valid (401 Unauthorized). Periksa kembali API Key Anda di Pengaturan.".to_string());
                } else {
                    let err_text = resp.text().await.unwrap_or_default();
                    tracing::warn!("Test ping on model {} failed (status {}): {}, trying next...", spec.id, status, err_text);
                    last_error = format!("Model {} ({})", spec.id, status);
                }
            }
            Err(e) => {
                last_error = format!("Koneksi ke {} gagal: {}", spec.id, e);
            }
        }
    }

    Err(format!(
        "Semua model gratis OpenRouter sedang sibuk atau tidak merespon. Terakhir: {}",
        last_error
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_delta() {
        let line = r#"data: {"model":"dots-studio/dots-3-note-preview:free","choices":[{"delta":{"content":"Halo"},"finish_reason":null}]}"#;
        let payload = parse_stream_line(line).unwrap();
        assert_eq!(payload.content, "Halo");
        assert_eq!(payload.model.as_deref(), Some("dots-studio/dots-3-note-preview:free"));
        assert!(!payload.done);
        assert!(payload.error.is_none());
    }

    #[test]
    fn parses_done_and_ignores_keepalives() {
        assert!(parse_stream_line(": OPENROUTER PROCESSING").is_none());
        assert!(parse_stream_line("").is_none());
        assert!(parse_stream_line("data: [DONE]").unwrap().done);
    }

    #[test]
    fn parses_stream_error() {
        let line = r#"data: {"error":{"message":"rate limited","code":429}}"#;
        let payload = parse_stream_line(line).unwrap();
        assert_eq!(payload.error.as_deref(), Some("rate limited"));
    }

    #[test]
    fn token_budgets_are_per_category() {
        assert_eq!(max_tokens_for("learning_dictionary"), 1000);
        assert_eq!(max_tokens_for("word_explanation"), 400);
        assert_eq!(max_tokens_for("unknown"), 700);
    }

    #[test]
    fn dots_and_laguna_disable_reasoning() {
        assert_eq!(FREE_MODEL_CHAIN[0].id, "dots-studio/dots-3-note-preview:free");
        assert_eq!(FREE_MODEL_CHAIN[0].effort, "none");
        assert_eq!(FREE_MODEL_CHAIN[1].id, "poolside/laguna-s-2.1:free");
        assert_eq!(FREE_MODEL_CHAIN[1].effort, "none");
        // MiniMax rejects `none`: reasoning is mandatory for that endpoint.
        assert_eq!(FREE_MODEL_CHAIN[2].effort, "low");
    }

    /// Manual end-to-end smoke test: set OPENROUTER_TOKEN and run with `--ignored`.
    /// It uses a throwaway database, so it cannot touch the app's real cache.
    #[tokio::test]
    #[ignore]
    async fn manual_live_stream() {
        use std::sync::Arc;
        use tauri::ipc::InvokeResponseBody;

        let Some(token) = std::env::var("OPENROUTER_TOKEN").ok().filter(|t| !t.trim().is_empty())
        else {
            eprintln!("OPENROUTER_TOKEN not set, skipping");
            return;
        };
        let path = std::env::temp_dir().join("limusic-ai-stream-test.sqlite");
        let _ = std::fs::remove_file(&path);
        let db = crate::db::Db::open(&path).unwrap();
        db.set_setting("openrouter_token", token.trim());

        let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let channel: Channel<AiStreamEvent> = Channel::new(move |body| {
            if let InvokeResponseBody::Json(json) = body {
                sink.lock().unwrap().push(json);
            }
            Ok(())
        });

        let req = AiExplainRequest {
            word: "serendipity".to_string(),
            sentence: "Finding that cafe was pure serendipity.".to_string(),
            category: "word_explanation".to_string(),
            system_prompt: "You are an English tutor. Explain the nuance of the word \"serendipity\" in the sentence \"Finding that cafe was pure serendipity.\" in Indonesian, maximum 2 sentences. Output only the explanation.".to_string(),
            user_prompt: "Explain \"serendipity\" in \"Finding that cafe was pure serendipity.\"".to_string(),
        };
        let started = Instant::now();
        query_ai_immersion_stream(&db, req, channel).await.unwrap();
        let events = events.lock().unwrap();
        println!("stream finished in {:?}, {} events", started.elapsed(), events.len());
        for event in events.iter() {
            println!("{event}");
        }
        assert!(events.iter().any(|e| e.contains("\"type\":\"delta\"")));
        assert!(events.iter().any(|e| e.contains("\"type\":\"done\"")));
    }
}
