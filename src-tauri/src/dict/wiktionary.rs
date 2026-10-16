//! Wiktionary REST API native parser (Wikimedia Foundation)
use std::time::Duration;
use regex::Regex;
use serde_json::Value;
use super::cambridge::{DictSense, ParsedDictEntry};

fn strip_html(html: &str) -> String {
    let re_script = Regex::new(r"(?is)<(script|style)[^>]*>.*?</(script|style)>").unwrap();
    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    let re_spaces = Regex::new(r"\s+").unwrap();
    let no_script = re_script.replace_all(html, " ");
    let no_tags = re_tags.replace_all(&no_script, " ");
    re_spaces.replace_all(&no_tags, " ").trim().to_string()
}

pub async fn lookup(client: &reqwest::Client, word: &str) -> Option<ParsedDictEntry> {
    let clean = word.trim().to_lowercase();
    if clean.is_empty() {
        return None;
    }

    let url = format!("https://en.wiktionary.org/api/rest_v1/page/definition/{}", urlencoding::encode(&clean));
    let res = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let json: Value = res.json().await.ok()?;
    let en_items = json.get("en")?.as_array()?;

    let mut senses = Vec::new();
    for item in en_items {
        let pos = item.get("partOfSpeech").and_then(|v| v.as_str()).unwrap_or("general").to_lowercase();
        if let Some(defs) = item.get("definitions").and_then(|v| v.as_array()) {
            for d in defs.iter().take(3) {
                if let Some(raw_def) = d.get("definition").and_then(|v| v.as_str()) {
                    let clean_def = strip_html(raw_def);
                    if !clean_def.is_empty() {
                        let example = d.get("examples")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|v| v.as_str())
                            .map(strip_html);
                        senses.push(DictSense {
                            pos: pos.clone(),
                            definition: clean_def,
                            example,
                        });
                    }
                }
            }
        }
    }

    if !senses.is_empty() {
        Some(ParsedDictEntry {
            word: clean,
            phonetic: None,
            audio_url: None,
            senses,
        })
    } else {
        None
    }
}
