//! Modular Native Dictionary & Translation Engine
pub mod cambridge;
pub mod oxford;
pub mod wiktionary;
pub mod datamuse;
pub mod translate;

use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
pub use cambridge::{DictSense, ParsedDictEntry};
use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictLookupResponse {
    pub word: String,
    pub phonetic: Option<String>,
    pub audio_url: Option<String>,
    pub senses: Vec<DictSense>,
    pub word_translation: Option<String>,
    pub sentence_translation: Option<String>,
    pub provider: String,
    pub from_cache: bool,
}

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(8)
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default()
    })
}

pub async fn lookup_word(
    db: &Db,
    word: &str,
    sentence: Option<String>,
    dict_provider: Option<String>,
    trans_provider: Option<String>,
    target_lang: Option<String>,
    auto_fallback: bool,
) -> Result<DictLookupResponse, String> {
    let clean_word = word.trim().to_lowercase();
    if clean_word.is_empty() {
        return Err("Kata tidak boleh kosong".to_string());
    }

    let dict_prov = dict_provider.unwrap_or_else(|| "cambridge".to_string()).to_lowercase();
    let trans_prov = trans_provider.unwrap_or_else(|| "google".to_string()).to_lowercase();
    let lang = target_lang.unwrap_or_else(|| "id".to_string()).to_lowercase();

    // 1. Check SQLite Persistent Cache first
    if let Some(cached_json) = db.get_dict_cache(&clean_word, &lang, &dict_prov) {
        if let Ok(mut cached_res) = serde_json::from_str::<DictLookupResponse>(&cached_json) {
            cached_res.from_cache = true;
            // If new sentence context is passed and not translated in cache, translate sentence now
            if let Some(sent) = sentence {
                if !sent.trim().is_empty() && cached_res.sentence_translation.is_none() {
                    let client = get_client();
                    cached_res.sentence_translation = translate::translate_text(client, &sent, &trans_prov, &lang).await;
                }
            }
            return Ok(cached_res);
        }
    }

    let client = get_client();

    // 2. Run Dictionary Lookup & Translations in parallel
    let word_clone = clean_word.clone();
    let dict_prov_clone = dict_prov.clone();
    let dict_task = async move {
        fetch_with_fallback(client, &word_clone, &dict_prov_clone, auto_fallback).await
    };

    let word_for_trans = clean_word.clone();
    let trans_prov_clone = trans_prov.clone();
    let lang_clone = lang.clone();
    let word_trans_task = async move {
        translate::translate_text(client, &word_for_trans, &trans_prov_clone, &lang_clone).await
    };

    let sent_text = sentence.clone().unwrap_or_default();
    let trans_prov_sent = trans_prov.clone();
    let lang_sent = lang.clone();
    let sent_trans_task = async move {
        if sent_text.trim().is_empty() {
            None
        } else {
            translate::translate_text(client, &sent_text, &trans_prov_sent, &lang_sent).await
        }
    };

    let (dict_res, word_tr, sent_tr) = tokio::join!(dict_task, word_trans_task, sent_trans_task);

    let (entry, actual_provider) = dict_res.unwrap_or_else(|| {
        (
            ParsedDictEntry {
                word: clean_word.clone(),
                phonetic: None,
                audio_url: None,
                senses: Vec::new(),
            },
            dict_prov.clone(),
        )
    });

    let response = DictLookupResponse {
        word: clean_word.clone(),
        phonetic: entry.phonetic,
        audio_url: entry.audio_url,
        senses: entry.senses,
        word_translation: word_tr,
        sentence_translation: sent_tr,
        provider: actual_provider,
        from_cache: false,
    };

    // Save to SQLite Persistent Cache if we found valid data
    if !response.senses.is_empty() || response.word_translation.is_some() {
        if let Ok(json_str) = serde_json::to_string(&response) {
            let _ = db.save_dict_cache(&clean_word, &lang, &dict_prov, &json_str);
        }
    }

    Ok(response)
}

async fn fetch_with_fallback(
    client: &reqwest::Client,
    word: &str,
    preferred_provider: &str,
    auto_fallback: bool,
) -> Option<(ParsedDictEntry, String)> {
    // 1. Try preferred provider first
    let res = match preferred_provider {
        "cambridge" => cambridge::lookup(client, word).await.map(|e| (e, "Cambridge Dictionary".to_string())),
        "oxford" => oxford::lookup(client, word).await.map(|e| (e, "Oxford Learner's Dictionary".to_string())),
        "wiktionary" => wiktionary::lookup(client, word).await.map(|e| (e, "Wiktionary".to_string())),
        "datamuse" | "wordnet" => datamuse::lookup(client, word).await.map(|e| (e, "WordNet".to_string())),
        _ => cambridge::lookup(client, word).await.map(|e| (e, "Cambridge Dictionary".to_string())),
    };

    if let Some((entry, prov)) = res {
        if !entry.senses.is_empty() {
            return Some((entry, prov));
        }
    }

    if !auto_fallback {
        return None;
    }

    // 2. Fallback Chain: Cambridge -> Wiktionary -> Datamuse -> Oxford
    let chain = match preferred_provider {
        "cambridge" => vec!["wiktionary", "datamuse", "oxford"],
        "oxford" => vec!["cambridge", "wiktionary", "datamuse"],
        "wiktionary" => vec!["datamuse", "cambridge", "oxford"],
        _ => vec!["wiktionary", "cambridge", "oxford"],
    };

    for prov in chain {
        let fallback_res = match prov {
            "cambridge" => cambridge::lookup(client, word).await.map(|e| (e, "Cambridge Dictionary".to_string())),
            "oxford" => oxford::lookup(client, word).await.map(|e| (e, "Oxford Learner's Dictionary".to_string())),
            "wiktionary" => wiktionary::lookup(client, word).await.map(|e| (e, "Wiktionary".to_string())),
            "datamuse" => datamuse::lookup(client, word).await.map(|e| (e, "WordNet".to_string())),
            _ => None,
        };

        if let Some((entry, name)) = fallback_res {
            if !entry.senses.is_empty() {
                return Some((entry, name));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_native_wiktionary_lookup() {
        let client = get_client();
        let res = wiktionary::lookup(client, "serendipity").await;
        assert!(res.is_some(), "Wiktionary should find 'serendipity'");
        let entry = res.unwrap();
        assert!(!entry.senses.is_empty(), "Senses should not be empty");
    }

    #[tokio::test]
    #[ignore]
    async fn test_native_cambridge_lookup() {
        let client = get_client();
        let res = cambridge::lookup(client, "serendipity").await;
        if let Some(entry) = res {
            assert!(!entry.senses.is_empty(), "Cambridge senses should not be empty");
            assert!(entry.phonetic.is_some(), "Cambridge should have phonetic IPA");
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_native_oxford_lookup() {
        let client = get_client();
        let res = oxford::lookup(client, "serendipity").await;
        if let Some(entry) = res {
            assert!(!entry.senses.is_empty(), "Oxford senses should not be empty");
        }
    }
}

