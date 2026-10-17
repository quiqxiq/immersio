use std::process::Command;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;

fn get_tts_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Split a sentence into chunks of maximum `max_len` characters, preferring punctuation/whitespace boundaries.
fn split_text_chunks(text: &str, max_len: usize) -> Vec<String> {
    let text = text.trim();
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    // Split by commas, semicolons, dashes, or words
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

/// Synthesize speech from text using high quality natural audio with offline fallback.
/// Returns a base64 Data URL ready for instant playback in HTML5 Audio.
pub async fn synthesize_speech(text: String, voice: String, _speed: f32) -> Result<String, String> {
    let clean = text.trim().to_string();
    if clean.is_empty() {
        return Err("Teks kalimat kosong".into());
    }

    let cache_key = format!("{}:{}", voice, clean);
    if let Ok(cache) = get_tts_cache().lock() {
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(cached.clone());
        }
    }

    // Determine target language code for natural neural voice
    let tl = match voice.to_lowercase().as_str() {
        v if v.contains("gb") || v.contains("uk") || v.contains("british") || v.starts_with("bf_") || v.starts_with("bm_") => "en-GB",
        v if v.contains("au") || v.contains("australian") => "en-AU",
        v if v.contains("espeak") || v.contains("offline") => "offline",
        _ => "en",
    };

    if tl != "offline" {
        // Try fetching natural audio via HTTP
        match fetch_natural_speech(&clean, tl).await {
            Ok(data_url) => {
                if let Ok(mut cache) = get_tts_cache().lock() {
                    // Limit in-memory cache to 500 items
                    if cache.len() > 500 {
                        cache.clear();
                    }
                    cache.insert(cache_key, data_url.clone());
                }
                return Ok(data_url);
            }
            Err(err) => {
                tracing::warn!(?err, "Natural speech request failed, falling back to local espeak-ng");
            }
        }
    }

    // Fallback: Local espeak-ng synthesis (works 100% offline on Linux)
    synthesize_espeak(&clean)
}

/// Fetch natural speech chunks and concatenate them into a single valid MP3 data URL.
async fn fetch_natural_speech(text: &str, tl: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
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
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("TTS endpoint returned status: {}", resp.status()));
        }

        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        combined_mp3.extend_from_slice(&bytes);
    }

    if combined_mp3.is_empty() {
        return Err("No audio bytes received".into());
    }

    let b64 = BASE64.encode(&combined_mp3);
    Ok(format!("data:audio/mp3;base64,{}", b64))
}

/// Local offline synthesis using espeak-ng
fn synthesize_espeak(text: &str) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("limusic_tts_{}.wav", std::process::id()));

    let output = Command::new("espeak-ng")
        .args(["-w", temp_file.to_str().unwrap_or("/tmp/limusic_tts.wav"), text])
        .output()
        .map_err(|e| format!("espeak-ng execution failed: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "espeak-ng exited with status: {}",
            output.status
        ));
    }

    let wav_bytes = std::fs::read(&temp_file)
        .map_err(|e| format!("Failed to read generated wav file: {}", e))?;

    // Cleanup temp file
    let _ = std::fs::remove_file(&temp_file);

    let b64 = BASE64.encode(&wav_bytes);
    Ok(format!("data:audio/wav;base64,{}", b64))
}
