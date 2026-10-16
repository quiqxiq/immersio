//! Translation services in native Rust
use std::time::Duration;
use serde_json::Value;

pub async fn translate_text(
    client: &reqwest::Client,
    text: &str,
    provider: &str,
    target_lang: &str,
) -> Option<String> {
    let clean = text.trim();
    if clean.is_empty() {
        return None;
    }

    match provider.to_lowercase().as_str() {
        "mymemory" => {
            if let Some(res) = translate_mymemory(client, clean, target_lang).await {
                return Some(res);
            }
            // Fallback to Google
            translate_google(client, clean, target_lang).await
        }
        "lingva" => {
            if let Some(res) = translate_lingva(client, clean, target_lang).await {
                return Some(res);
            }
            // Fallback to Google
            translate_google(client, clean, target_lang).await
        }
        _ => translate_google(client, clean, target_lang).await,
    }
}

async fn translate_google(client: &reqwest::Client, text: &str, target_lang: &str) -> Option<String> {
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}",
        target_lang,
        urlencoding::encode(text)
    );

    let res = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let json: Value = res.json().await.ok()?;
    let parts = json.as_array()?.first()?.as_array()?;

    let mut out = String::new();
    for p in parts {
        if let Some(seg) = p.as_array().and_then(|arr| arr.first()).and_then(|v| v.as_str()) {
            out.push_str(seg);
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

async fn translate_mymemory(client: &reqwest::Client, text: &str, target_lang: &str) -> Option<String> {
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair=en|{}",
        urlencoding::encode(text),
        target_lang
    );

    let res = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let json: Value = res.json().await.ok()?;
    let trans = json
        .get("responseData")?
        .get("translatedText")?
        .as_str()?
        .trim();

    if trans.is_empty() || trans == text {
        None
    } else {
        Some(trans.to_string())
    }
}

async fn translate_lingva(client: &reqwest::Client, text: &str, target_lang: &str) -> Option<String> {
    let url = format!(
        "https://lingva.ml/api/v1/auto/{}/{}",
        target_lang,
        urlencoding::encode(text)
    );

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

    let json: Value = res.json().await.ok()?;
    json.get("translation")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
}
