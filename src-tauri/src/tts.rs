use std::process::Command;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;
use kothok_edge_tts::{init_tls, EdgeTts, Engine as EdgeEngine, TtsEvent};

fn get_tts_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ensure_tls_initialized() {
    static TLS_INIT: OnceLock<()> = OnceLock::new();
    TLS_INIT.get_or_init(|| {
        init_tls();
    });
}

/// Convert speed multiplier (e.g. 1.0, 1.15, 0.85) to SSML rate string (e.g. "+0%", "+15%", "-15%")
fn format_rate(speed: f32) -> String {
    let pct = ((speed - 1.0) * 100.0).round() as i32;
    if pct >= 0 {
        format!("+{}%", pct)
    } else {
        format!("{}%", pct)
    }
}

/// Synthesize speech from text using the Hybrid Architecture:
/// 1. Primary: Microsoft Edge Neural TTS (Studio-grade human voice, 10/10 naturalness)
/// 2. Secondary fallback: Google TTS (Quick online fallback)
/// 3. Offline fallback: Local espeak-ng (100% offline Linux synthesis)
pub async fn synthesize_speech(text: String, voice: String, speed: f32) -> Result<String, String> {
    let clean = text.trim().to_string();
    if clean.is_empty() {
        return Err("Teks kalimat kosong".into());
    }

    let cache_key = format!("{}:{}:{}", voice, speed, clean);
    if let Ok(cache) = get_tts_cache().lock() {
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(cached.clone());
        }
    }

    let is_explicit_offline = voice.to_lowercase().contains("offline") || voice.to_lowercase().contains("espeak");

    if !is_explicit_offline {
        // --- Tier 1: Microsoft Edge Neural TTS ---
        ensure_tls_initialized();
        let edge_voice = match voice.as_str() {
            "en-US-JennyNeural" | "jenny" | "en" => "en-US-JennyNeural",
            "en-US-GuyNeural" | "guy" => "en-US-GuyNeural",
            "en-US-AriaNeural" | "aria" => "en-US-AriaNeural",
            "en-GB-SoniaNeural" | "sonia" | "en-GB" => "en-GB-SoniaNeural",
            "en-GB-RyanNeural" | "ryan" => "en-GB-RyanNeural",
            "en-AU-NatashaNeural" | "natasha" | "en-AU" => "en-AU-NatashaNeural",
            v if v.contains("Neural") => v,
            _ => "en-US-JennyNeural",
        };

        let lang = if edge_voice.starts_with("en-GB") {
            "en-GB"
        } else if edge_voice.starts_with("en-AU") {
            "en-AU"
        } else {
            "en-US"
        };

        let rate = format_rate(speed);

        tracing::info!(edge_voice, rate = %rate, "Synthesizing with Edge Neural TTS");
        match EdgeTts.synthesize(&clean, edge_voice, &rate, lang).await {
            Ok(events) => {
                let mut mp3_bytes = Vec::new();
                for event in events {
                    if let TtsEvent::Audio(bytes) = event {
                        mp3_bytes.extend(bytes);
                    }
                }

                if !mp3_bytes.is_empty() {
                    let b64 = BASE64.encode(&mp3_bytes);
                    let data_url = format!("data:audio/mp3;base64,{}", b64);
                    save_to_cache(cache_key, data_url.clone());
                    return Ok(data_url);
                }
            }
            Err(err) => {
                tracing::warn!(?err, "Edge Neural TTS failed, trying Tier 2 Google TTS");
            }
        }

        // --- Tier 2: Google TTS Fallback ---
        let tl = if edge_voice.starts_with("en-GB") { "en-GB" } else { "en" };
        if let Ok(data_url) = fetch_google_tts(&clean, tl).await {
            save_to_cache(cache_key, data_url.clone());
            return Ok(data_url);
        }
    }

    // --- Tier 3: Local Offline espeak-ng Fallback ---
    tracing::info!("Using local offline espeak-ng for TTS");
    match synthesize_espeak(&clean) {
        Ok(data_url) => {
            save_to_cache(cache_key, data_url.clone());
            Ok(data_url)
        }
        Err(e) => Err(format!("Gagal menyuarakan audio (Online & Offline gagal): {}", e)),
    }
}

fn save_to_cache(key: String, val: String) {
    if let Ok(mut cache) = get_tts_cache().lock() {
        if cache.len() > 600 {
            cache.clear();
        }
        cache.insert(key, val);
    }
}

/// Split text into <= 150 char chunks for Google TTS
fn split_text_chunks(text: &str, max_len: usize) -> Vec<String> {
    let text = text.trim();
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_len {
            current.push(' ');
            current.push_str(word);
        } else {
            chunks.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Secondary online fallback using Google TTS
async fn fetch_google_tts(text: &str, tl: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|e| e.to_string())?;

    let chunks = split_text_chunks(text, 150);
    let mut combined_mp3: Vec<u8> = Vec::new();

    for chunk in chunks {
        if chunk.trim().is_empty() {
            continue;
        }

        let encoded = urlencoding::encode(chunk.trim());
        let url = format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&tl={}&client=tw-ob&q={}",
            tl, encoded
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
            .header("Referer", "https://translate.google.com/")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Status {}", resp.status()));
        }

        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        combined_mp3.extend_from_slice(&bytes);
    }

    if combined_mp3.is_empty() {
        return Err("Empty response".into());
    }

    let b64 = BASE64.encode(&combined_mp3);
    Ok(format!("data:audio/mp3;base64,{}", b64))
}

/// Tier 3: Local offline synthesis using espeak-ng
fn synthesize_espeak(text: &str) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("limusic_tts_{}.wav", std::process::id()));

    let output = Command::new("espeak-ng")
        .args(["-w", temp_file.to_str().unwrap_or("/tmp/limusic_tts.wav"), text])
        .output()
        .map_err(|e| format!("espeak-ng execution failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("espeak-ng exit status: {}", output.status));
    }

    let wav_bytes = std::fs::read(&temp_file)
        .map_err(|e| format!("Failed to read wav: {}", e))?;

    let _ = std::fs::remove_file(&temp_file);

    let b64 = BASE64.encode(&wav_bytes);
    Ok(format!("data:audio/wav;base64,{}", b64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_edge_tts_synthesis() {
        let result = synthesize_speech("Hello world, this is a test of Edge Neural TTS.".to_string(), "en-US-JennyNeural".to_string(), 1.0).await;
        assert!(result.is_ok(), "Expected Ok but got: {:?}", result.err());
        let data_url = result.unwrap();
        assert!(data_url.starts_with("data:audio/mp3;base64,"));
        assert!(data_url.len() > 500);
    }
}
