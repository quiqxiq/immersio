//! Native AnkiConnect client for Limusic.
//!
//! 100% Rust implementation communicating directly with Anki Desktop's AnkiConnect add-on
//! via JSON-RPC at `http://127.0.0.1:8765`. No Python daemon needed.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

pub const DEFAULT_ANKI_PORT: u16 = 8765;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiStatus {
    pub connected: bool,
    pub version: Option<i64>,
    pub port: u16,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineCardRequest {
    pub word: String,
    pub lemma: String,
    pub sentence: String,
    pub translation: String,
    pub source_title: String,
    pub source_url: String,
    pub deck_name: Option<String>,
    pub model_name: Option<String>,
    pub audio_url: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineCardResponse {
    pub success: bool,
    pub anki_note_id: Option<i64>,
    pub message: String,
}

#[derive(Serialize)]
struct AnkiRpcRequest<'a> {
    action: &'a str,
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct AnkiRpcResponse<T> {
    result: Option<T>,
    error: Option<String>,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default()
}

fn anki_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

async fn invoke<T: for<'de> Deserialize<'de>>(
    action: &str,
    params: Option<serde_json::Value>,
    port: u16,
) -> Result<T, String> {
    let req = AnkiRpcRequest {
        action,
        version: 6,
        params,
    };

    let res = client()
        .post(anki_url(port))
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Koneksi ke AnkiConnect gagal ({e}). Pastikan Anki Desktop terbuka."))?;

    let parsed: AnkiRpcResponse<T> = res
        .json()
        .await
        .map_err(|e| format!("Gagal memproses respon AnkiConnect: {e}"))?;

    if let Some(err) = parsed.error {
        return Err(err);
    }

    parsed.result.ok_or_else(|| "Respon kosong dari AnkiConnect".to_string())
}

/// Check if AnkiConnect is running and responsive.
pub async fn check_status(port: Option<u16>) -> AnkiStatus {
    let p = port.unwrap_or(DEFAULT_ANKI_PORT);
    match invoke::<i64>("version", None, p).await {
        Ok(v) => AnkiStatus {
            connected: true,
            version: Some(v),
            port: p,
            error: None,
        },
        Err(e) => AnkiStatus {
            connected: false,
            version: None,
            port: p,
            error: Some(e),
        },
    }
}

/// Get all deck names in Anki.
pub async fn get_decks(port: Option<u16>) -> Result<Vec<String>, String> {
    let p = port.unwrap_or(DEFAULT_ANKI_PORT);
    invoke::<Vec<String>>("deckNames", None, p).await
}

/// Get all note model names in Anki.
pub async fn get_models(port: Option<u16>) -> Result<Vec<String>, String> {
    let p = port.unwrap_or(DEFAULT_ANKI_PORT);
    invoke::<Vec<String>>("modelNames", None, p).await
}

/// Mine a card directly to Anki Desktop.
pub async fn mine_card(req: MineCardRequest, port: Option<u16>) -> Result<MineCardResponse, String> {
    let p = port.unwrap_or(DEFAULT_ANKI_PORT);

    // Default to first deck or "LinguaMiner" / "English"
    let decks = get_decks(Some(p)).await.unwrap_or_default();
    let deck = req.deck_name.clone().unwrap_or_else(|| {
        if decks.iter().any(|d| d.eq_ignore_ascii_case("English")) {
            "English".to_string()
        } else if decks.iter().any(|d| d.eq_ignore_ascii_case("LinguaMiner")) {
            "LinguaMiner".to_string()
        } else {
            decks.first().cloned().unwrap_or_else(|| "Default".to_string())
        }
    });

    let models = get_models(Some(p)).await.unwrap_or_default();
    let model = req.model_name.clone().unwrap_or_else(|| {
        if models.iter().any(|m| m.eq_ignore_ascii_case("Basic")) {
            "Basic".to_string()
        } else {
            models.first().cloned().unwrap_or_else(|| "Basic".to_string())
        }
    });

    // Format Front and Back HTML
    // Front: Word in bold, context sentence with target word highlighted
    let front = format!(
        "<div style='font-size: 24px; font-weight: bold; color: #2563eb; margin-bottom: 8px;'>{}</div>\
         <div style='font-size: 16px; line-height: 1.5; color: #374151;'>{}</div>",
        html_escape(&req.word),
        highlight_target_word(&req.sentence, &req.word)
    );

    // Back: Translation and source
    let mut back = format!(
        "<div style='font-size: 18px; font-weight: 600; color: #111827; margin-bottom: 6px;'>{}</div>",
        html_escape(&req.translation)
    );

    if !req.source_title.is_empty() {
        back.push_str(&format!(
            "<div style='font-size: 12px; color: #9ca3af; margin-top: 12px;'>Sumber: {}</div>",
            html_escape(&req.source_title)
        ));
    }

    // Build fields according to model
    let fields = serde_json::json!({
        "Front": front,
        "Back": back
    });

    let note_payload = serde_json::json!({
        "note": {
            "deckName": deck,
            "modelName": model,
            "fields": fields,
            "options": {
                "allowDuplicate": false,
                "duplicateScope": "deck"
            },
            "tags": ["limusic", "english-mining"]
        }
    });

    match invoke::<i64>("addNote", Some(note_payload), p).await {
        Ok(note_id) => Ok(MineCardResponse {
            success: true,
            anki_note_id: Some(note_id),
            message: format!("Kartu berhasil ditambahkan ke deck '{deck}'!"),
        }),
        Err(e) => {
            if e.contains("cannot create note because it is a duplicate") {
                Ok(MineCardResponse {
                    success: false,
                    anki_note_id: None,
                    message: "Kata ini sudah ada di dalam Anki deck Anda (duplikat).".to_string(),
                })
            } else {
                Err(format!("Gagal menambahkan kartu ke Anki: {e}"))
            }
        }
    }
}

/// Query Anki card intervals for a list of note IDs.
/// Returns mapping of note_id -> interval (in days).
pub async fn get_note_intervals(
    note_ids: Vec<i64>,
    port: Option<u16>,
) -> Result<HashMap<i64, i64>, String> {
    let p = port.unwrap_or(DEFAULT_ANKI_PORT);
    if note_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let payload = serde_json::json!({
        "notes": note_ids
    });

    #[derive(Deserialize)]
    struct NoteCardInfo {
        cards: Option<Vec<i64>>,
    }

    let notes_info: Vec<NoteCardInfo> = invoke("notesInfo", Some(payload), p).await?;
    let mut card_ids = Vec::new();
    let mut card_to_note = HashMap::new();

    for (i, note) in notes_info.into_iter().enumerate() {
        if let Some(cards) = note.cards {
            if let Some(first_card) = cards.first() {
                if let Some(&nid) = note_ids.get(i) {
                    card_ids.push(*first_card);
                    card_to_note.insert(*first_card, nid);
                }
            }
        }
    }

    if card_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(Deserialize)]
    struct CardInfo {
        #[serde(rename = "cardId")]
        card_id: i64,
        interval: i64,
    }

    let cards_payload = serde_json::json!({
        "cards": card_ids
    });

    let cards_info: Vec<CardInfo> = invoke("cardsInfo", Some(cards_payload), p).await?;
    let mut result = HashMap::new();

    for c in cards_info {
        if let Some(&nid) = card_to_note.get(&c.card_id) {
            result.insert(nid, c.interval);
        }
    }

    Ok(result)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn highlight_target_word(sentence: &str, target: &str) -> String {
    let escaped = html_escape(sentence);
    let target_escaped = html_escape(target);
    if target_escaped.is_empty() {
        return escaped;
    }

    // Case-insensitive replace for target word with bold underline
    let re = regex::RegexBuilder::new(&format!(r"(?i)\b{}\b", regex::escape(&target_escaped)))
        .build();

    if let Ok(re) = re {
        re.replace_all(&escaped, "<b style='color: #2563eb; text-decoration: underline;'>$0</b>")
            .to_string()
    } else {
        escaped
    }
}
