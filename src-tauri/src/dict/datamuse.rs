//! Datamuse WordNet API client
use std::time::Duration;
use serde_json::Value;
use super::cambridge::{DictSense, ParsedDictEntry};

pub async fn lookup(client: &reqwest::Client, word: &str) -> Option<ParsedDictEntry> {
    let clean = word.trim().to_lowercase();
    if clean.is_empty() {
        return None;
    }

    let url = format!("https://api.datamuse.com/words?sp={}&md=dprf", urlencoding::encode(&clean));
    let res = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let list: Value = res.json().await.ok()?;
    let item = list.as_array()?.first()?;

    let mut phonetic = None;
    if let Some(tags) = item.get("tags").and_then(|v| v.as_array()) {
        for t in tags {
            if let Some(s) = t.as_str() {
                if s.starts_with("pron:") {
                    phonetic = Some(format!("/{}", s.replace("pron:", "").trim().to_lowercase()));
                    break;
                }
            }
        }
    }

    let mut senses = Vec::new();
    if let Some(defs) = item.get("defs").and_then(|v| v.as_array()) {
        for d in defs.iter().take(4) {
            if let Some(d_str) = d.as_str() {
                let parts: Vec<&str> = d_str.split('\t').collect();
                let raw_pos = if parts.len() > 1 { parts[0] } else { "general" };
                let text = if parts.len() > 1 { parts[1].trim() } else { parts[0].trim() };
                let pos = match raw_pos {
                    "n" => "noun",
                    "v" => "verb",
                    "adj" => "adjective",
                    "adv" => "adverb",
                    other => other,
                };
                if !text.is_empty() {
                    senses.push(DictSense {
                        pos: pos.to_string(),
                        definition: text.to_string(),
                        example: None,
                    });
                }
            }
        }
    }

    if !senses.is_empty() {
        Some(ParsedDictEntry {
            word: clean,
            phonetic,
            audio_url: None,
            senses,
        })
    } else {
        None
    }
}
