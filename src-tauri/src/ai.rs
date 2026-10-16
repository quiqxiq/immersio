//! OpenRouter Multi-Model AI Language Immersion Engine
//!
//! Provides intelligent contextual explanation, learning dictionary generation,
//! and sentence structure breakdowns with automatic multi-model failover
//! and output sanitization.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use crate::db::Db;

pub const FREE_MODEL_CHAIN: &[&str] = &[
    "liquid/lfm-2.5-2.6b:free",
    "nvidia/nemotron-3-super-120b-a12b:free",
    "dots-studio/dots-3-note-preview:free",
    "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free",
    "poolside/laguna-s-2.1:free",
];

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(8)
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .unwrap_or_default()
    })
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

#[derive(Debug, Serialize)]
struct OpenRouterMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct OpenRouterRequestBody<'a> {
    model: &'a str,
    messages: Vec<OpenRouterMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
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
struct OpenRouterModelsList {
    data: Option<Vec<OpenRouterModelInfo>>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelInfo {
    id: String,
    pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

/// Dynamically fetch active free models from OpenRouter's live API,
/// strictly filtering out safety classifiers and moderation tools.
pub async fn fetch_dynamic_free_models() -> Vec<String> {
    let client = get_client();
    let resp = match client.get("https://openrouter.ai/api/v1/models").send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    if !resp.status().is_success() {
        return Vec::new();
    }

    let data: OpenRouterModelsList = match resp.json().await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut models = Vec::new();
    if let Some(items) = data.data {
        for item in items {
            let id_lower = item.id.to_lowercase();
            // Strictly exclude safety classifiers, guards, and embedding models
            if id_lower.contains("safety")
                || id_lower.contains("guard")
                || id_lower.contains("moderation")
                || id_lower.contains("embed")
                || id_lower.contains("rerank")
                || id_lower == "openrouter/free"
            {
                continue;
            }

            let is_free_id = item.id.ends_with(":free") && item.id != "openrouter/free";
            let is_free_price = item.pricing.as_ref().map_or(false, |p| {
                p.prompt.as_deref() == Some("0") && p.completion.as_deref() == Some("0")
            });
            if is_free_id || is_free_price {
                models.push(item.id);
            }
        }
    }
    models
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenRouterErrorDetail {
    message: Option<String>,
    code: Option<serde_json::Value>,
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

/// Queries OpenRouter using the multi-model fallback chain.
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
            return Ok(AiExplainResponse {
                content,
                model_used,
                from_cache: true,
            });
        }
    }

    let client = get_client();
    let mut last_error = String::new();

    // 2. Iterate through fallback chain (curated chain first, then live dynamic discovery if needed)
    let mut models_to_try: Vec<String> = FREE_MODEL_CHAIN.iter().map(|&s| s.to_string()).collect();
    let mut tried = std::collections::HashSet::new();

    let mut i = 0;
    while i < models_to_try.len() {
        let model = models_to_try[i].clone();
        i += 1;
        if tried.contains(&model) {
            continue;
        }
        tried.insert(model.clone());

        let body = OpenRouterRequestBody {
            model: &model,
            messages: vec![
                OpenRouterMessage {
                    role: "system",
                    content: &req.system_prompt,
                },
                OpenRouterMessage {
                    role: "user",
                    content: &req.user_prompt,
                },
            ],
            temperature: 0.2,
            max_tokens: 2500,
        };

        let response_res = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", token))
            .header("HTTP-Referer", "https://github.com/lingua-miner/limusic")
            .header("X-Title", "Limusic Immersion")
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(15))
            .json(&body)
            .send()
            .await;

        match response_res {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    let resp_body: Result<OpenRouterResponse, _> = resp.json().await;
                    if let Ok(data) = resp_body {
                        let actual_model = data.model.unwrap_or_else(|| model.clone());
                        if let Some(choices) = data.choices {
                            if let Some(first_choice) = choices.into_iter().next() {
                                let raw_text = first_choice.message.and_then(|m| {
                                    m.content
                                        .filter(|c| !c.trim().is_empty())
                                        .or(m.reasoning.filter(|r| !r.trim().is_empty()))
                                });

                                if let Some(content) = raw_text {
                                    let cleaned = clean_ai_output(&content);
                                    if is_valid_immersion_output(&cleaned) {
                                        // Cache result in SQLite
                                        let _ = db.save_ai_cache(
                                            &cache_key,
                                            &req.word,
                                            &req.sentence,
                                            &req.category,
                                            &actual_model,
                                            &cleaned,
                                        );

                                        return Ok(AiExplainResponse {
                                            content: cleaned,
                                            model_used: actual_model,
                                            from_cache: false,
                                        });
                                    } else {
                                        tracing::warn!(
                                            "Model {} returned invalid/safety content: {:?}, trying next fallback...",
                                            model,
                                            cleaned
                                        );
                                        last_error = format!("Model {} mengembalikan konten tidak valid (safety/terlalu pendek)", model);
                                    }
                                }
                            }
                        }
                    }
                } else if status.as_u16() == 401 {
                    return Err("Token OpenRouter tidak valid (401 Unauthorized). Periksa kembali API Key Anda di Pengaturan.".to_string());
                } else {
                    let err_text = resp.text().await.unwrap_or_default();
                    tracing::warn!("OpenRouter model {} returned status {}: {}, trying next fallback...", model, status, err_text);
                    last_error = format!("Model {} gagal (status {}): {}", model, status, err_text);
                }
            }
            Err(e) => {
                tracing::warn!("OpenRouter request error on model {}: {}, trying next fallback...", model, e);
                last_error = format!("Koneksi ke {} gagal: {}", model, e);
            }
        }

        // If curated models were exhausted without success, fetch live available free models from OpenRouter
        if i == FREE_MODEL_CHAIN.len() && models_to_try.len() == FREE_MODEL_CHAIN.len() {
            let dynamic = fetch_dynamic_free_models().await;
            for dyn_model in dynamic {
                if !tried.contains(&dyn_model) {
                    models_to_try.push(dyn_model);
                }
            }
        }
    }

    Err(format!(
        "Semua model AI OpenRouter sedang sibuk atau tidak dapat dijangkau. Rincian terakhir: {}",
        last_error
    ))
}

/// Test if an OpenRouter token is valid by making a lightweight model ping.
pub async fn test_openrouter_connection(token: &str) -> Result<String, String> {
    let clean_token = token.trim();
    if clean_token.is_empty() {
        return Err("Token tidak boleh kosong.".to_string());
    }

    let client = get_client();
    let mut last_error = String::new();

    for &model in FREE_MODEL_CHAIN {
        let body = OpenRouterRequestBody {
            model,
            messages: vec![OpenRouterMessage {
                role: "user",
                content: "Say 'OK' only.",
            }],
            temperature: 0.1,
            max_tokens: 10,
        };

        let resp_res = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", clean_token))
            .header("HTTP-Referer", "https://github.com/lingua-miner/limusic")
            .header("X-Title", "Limusic Immersion")
            .header("Content-Type", "application/json")
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
                        .unwrap_or_else(|| model.to_string());
                    return Ok(actual_model);
                } else if status.as_u16() == 401 {
                    return Err("Token OpenRouter tidak valid (401 Unauthorized). Periksa kembali API Key Anda di Pengaturan.".to_string());
                } else {
                    let err_text = resp.text().await.unwrap_or_default();
                    tracing::warn!("Test ping on model {} failed (status {}): {}, trying next...", model, status, err_text);
                    last_error = format!("Model {} ({})", model, status);
                }
            }
            Err(e) => {
                last_error = format!("Koneksi ke {} gagal: {}", model, e);
            }
        }
    }

    Err(format!(
        "Semua model gratis OpenRouter sedang sibuk atau tidak merespon. Terakhir: {}",
        last_error
    ))
}
