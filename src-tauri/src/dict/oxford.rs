//! Oxford Learner's Dictionary native parser
use std::time::Duration;
use regex::Regex;
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

    let url = format!("https://www.oxfordlearnersdictionaries.com/definition/english/{}", urlencoding::encode(&clean));
    let res = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .timeout(Duration::from_secs(4))
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let html = res.text().await.ok()?;

    if html.contains("entry") || html.contains("webtop") || html.contains("class=\"def\"") {
        let re_pos = Regex::new(r#"<span class="pos"[^>]*>([^<]+)</span>"#).unwrap();
        let re_phon = Regex::new(r#"<span class="phon"[^>]*>([^<]+)</span>"#).unwrap();
        let re_def = Regex::new(r#"(?s)<span class="def"[^>]*>(.*?)</span>"#).unwrap();
        let re_ex = Regex::new(r#"(?s)<span class="x"[^>]*>(.*?)</span>"#).unwrap();
        let re_audio = Regex::new(r#"data-src-mp3="([^"]+)""#).unwrap();

        let pos_match = re_pos.captures(&html).and_then(|c| c.get(1)).map(|m| m.as_str().trim().to_string()).unwrap_or_else(|| "general".to_string());
        let phonetic = re_phon.captures(&html).and_then(|c| c.get(1)).map(|m| m.as_str().trim().to_string());
        let audio_url = re_audio.captures(&html).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());

        let mut examples: Vec<String> = Vec::new();
        for cap in re_ex.captures_iter(&html).take(5) {
            if let Some(m) = cap.get(1) {
                let clean_ex = strip_html(m.as_str());
                if !clean_ex.is_empty() {
                    examples.push(clean_ex);
                }
            }
        }

        let mut senses: Vec<DictSense> = Vec::new();
        for (i, cap) in re_def.captures_iter(&html).take(4).enumerate() {
            if let Some(m) = cap.get(1) {
                let clean_def = strip_html(m.as_str());
                if !clean_def.is_empty() {
                    let example = examples.get(i).cloned();
                    senses.push(DictSense {
                        pos: pos_match.clone(),
                        definition: clean_def,
                        example,
                    });
                }
            }
        }

        if !senses.is_empty() {
            return Some(ParsedDictEntry {
                word: clean,
                phonetic,
                audio_url,
                senses,
            });
        }
    }

    None
}
