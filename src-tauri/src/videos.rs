//! YouTube Video Hub Backend for Limusic
//!
//! Provides account-level YouTube integration:
//! - Subscriptions Feed (`FEsubscriptions`)
//! - Subscribed Channels Explorer (`FEchannels`)
//! - Channel Videos Browser (`<channelId>` with Videos tab)
//! - Personalized Home Recommendations (`FEwhat_to_watch`)
//! - Curated Non-Music Trending Categories (AI, Tech, Science, Podcasts, Coding)

use std::sync::OnceLock;
use std::time::Duration;

use innertube::{cookie_sapisid, sapisid_hash, InnerTube, SongItem, YouTubeClient};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::Db;

const YT_ORIGIN: &str = "https://www.youtube.com";
const YT_REFERER: &str = "https://www.youtube.com/";
const YT_BROWSE_URL: &str = "https://www.youtube.com/youtubei/v1/browse";

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(8)
            .build()
            .unwrap_or_default()
    })
}

/// Representation of a YouTube channel the user subscribed to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribedChannel {
    pub id: String,
    pub title: String,
    pub subscriber_count: String,
    pub video_count: Option<String>,
    pub avatar_url: String,
    pub description: Option<String>,
}

/// Builds request headers using the Google session cookie from SQLite.
fn build_yt_headers(cookie: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();

    let set = |h: &mut HeaderMap, k: &'static str, v: &str| {
        if let Ok(val) = HeaderValue::from_str(v) {
            h.insert(HeaderName::from_static(k), val);
        }
    };

    set(&mut headers, "content-type", "application/json");
    set(&mut headers, "accept", "application/json");
    set(&mut headers, "x-goog-api-format-version", "1");
    set(&mut headers, "x-youtube-client-name", "1");
    set(&mut headers, "x-youtube-client-version", "2.20240101.00.00");
    set(&mut headers, "x-origin", YT_ORIGIN);
    set(&mut headers, "origin", YT_ORIGIN);
    set(&mut headers, "referer", YT_REFERER);
    set(
        &mut headers,
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
    );

    let trimmed = cookie.trim();
    if !trimmed.is_empty() {
        set(&mut headers, "cookie", trimmed);
        if let Some(sapisid) = cookie_sapisid(trimmed) {
            let auth = sapisid_hash(sapisid, YT_ORIGIN);
            set(&mut headers, "authorization", &auth);
        }
    }

    headers
}

/// Performs a raw InnerTube browse request with YouTube Web client.
pub(crate) async fn post_browse(
    cookie: &str,
    browse_id: &str,
    params: Option<&str>,
    visitor_data: Option<&str>,
) -> Result<Value, String> {
    let client = get_client();
    let headers = build_yt_headers(cookie);

    let mut client_obj = serde_json::json!({
        "clientName": "WEB",
        "clientVersion": "2.20240101.00.00",
        "hl": "en",
        "gl": "US"
    });

    if let Some(vd) = visitor_data {
        if !vd.trim().is_empty() {
            client_obj["visitorData"] = serde_json::Value::String(vd.trim().to_string());
        }
    }

    let mut body = serde_json::json!({
        "context": {
            "client": client_obj
        },
        "browseId": browse_id
    });

    if let Some(p) = params {
        body["params"] = serde_json::Value::String(p.to_string());
    }

    let resp = client
        .post(YT_BROWSE_URL)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Koneksi ke YouTube gagal: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("YouTube error (status {status}): {err_text}"));
    }

    resp.json::<Value>()
        .await
        .map_err(|e| format!("Gagal membaca respons YouTube: {e}"))
}

/// Performs a raw InnerTube continuation browse request with YouTube Web client.
pub(crate) async fn post_browse_continuation(cookie: &str, token: &str) -> Result<Value, String> {
    let client = get_client();
    let headers = build_yt_headers(cookie);

    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": "2.20240101.00.00",
                "hl": "en",
                "gl": "US"
            }
        }
    });

    let url = format!("{YT_BROWSE_URL}?continuation={}", urlencoding::encode(token));

    let resp = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Koneksi ke YouTube gagal: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("YouTube error (status {status}): {err_text}"));
    }

    resp.json::<Value>()
        .await
        .map_err(|e| format!("Gagal membaca respons YouTube: {e}"))
}

/// Checks if a string looks like a video duration (e.g. "12:34", "1:02:15", "LIVE").
fn is_duration_str(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    if s.eq_ignore_ascii_case("live") {
        return true;
    }
    s.contains(':') && s.chars().all(|c| c.is_ascii_digit() || c == ':')
}

/// Extracts duration string from overlay badges (standard across modern and classic YouTube renderers).
fn extract_duration_from_overlays(val: &Value) -> Option<String> {
    if let Some(overlays) = val.as_array() {
        for o in overlays {
            // 1. Modern thumbnailBottomOverlayViewModel -> badges -> thumbnailBadgeViewModel -> text
            if let Some(badges) = o
                .get("thumbnailBottomOverlayViewModel")
                .and_then(|b| b.get("badges"))
                .and_then(|b| b.as_array())
            {
                for badge in badges {
                    if let Some(t) = badge
                        .get("thumbnailBadgeViewModel")
                        .and_then(|b| b.get("text"))
                    {
                        if let Some(s) = t.as_str() {
                            if is_duration_str(s) {
                                return Some(s.to_string());
                            }
                        } else if let Some(s) = t.get("content").and_then(|c| c.as_str()) {
                            if is_duration_str(s) {
                                return Some(s.to_string());
                            }
                        }
                    }
                }
            }

            // 2. Classic thumbnailOverlayTimeStatusRenderer -> text
            if let Some(text_val) = o
                .get("thumbnailOverlayTimeStatusRenderer")
                .and_then(|r| r.get("text"))
            {
                if let Some(s) = text_val.get("simpleText").and_then(|s| s.as_str()) {
                    if is_duration_str(s) {
                        return Some(s.to_string());
                    }
                }
                if let Some(runs) = text_val.get("runs").and_then(|r| r.as_array()) {
                    if let Some(s) = runs.first().and_then(|r| r.get("text")).and_then(|t| t.as_str()) {
                        if is_duration_str(s) {
                            return Some(s.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Normalizes channel names by stripping accessibility prefixes in various languages.
fn clean_channel_name(s: &str) -> String {
    let mut s = s.trim();
    for prefix in [
        "Go to channel ",
        "Go to ",
        "Buka channel ",
        "Buka ",
        "Kunjungi channel ",
        "Kunjungi ",
        "Channel ",
        "Saluran ",
    ] {
        if let Some(stripped) = s.strip_prefix(prefix) {
            s = stripped.trim();
        }
    }
    s.to_string()
}

/// Recursively or flexibly extracts plain text from any InnerTube text container
/// (string, content, simpleText, or runs array).
fn extract_text_val(val: &Value) -> Option<String> {
    if let Some(s) = val.as_str() {
        let t = s.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Some(s) = val.get("content").and_then(|c| c.as_str()) {
        let t = s.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Some(s) = val.get("simpleText").and_then(|c| c.as_str()) {
        let t = s.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Some(runs) = val.get("runs").and_then(|r| r.as_array()) {
        let parts: Vec<String> = runs
            .iter()
            .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !parts.is_empty() {
            return Some(parts.join(" "));
        }
    }
    None
}

/// Recursively traverses a JSON value to collect YouTube filter chips (`chipCloudChipRenderer`).
pub fn extract_chips(val: &Value, out: &mut Vec<innertube::HomeChip>) {
    match val {
        Value::Object(map) => {
            if let Some(chip) = map.get("chipCloudChipRenderer") {
                let title = chip
                    .get("text")
                    .and_then(extract_text_val)
                    .unwrap_or_default();
                let params = chip
                    .get("navigationEndpoint")
                    .and_then(|n| {
                        n.get("browseEndpoint")
                            .and_then(|b| b.get("params"))
                            .or_else(|| n.get("continuationCommand").and_then(|c| c.get("token")))
                    })
                    .and_then(|p| p.as_str())
                    .unwrap_or_default()
                    .to_string();
                if !title.is_empty() && !out.iter().any(|c| c.title == title) {
                    out.push(innertube::HomeChip { title, params });
                }
            }
            for v in map.values() {
                extract_chips(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                extract_chips(v, out);
            }
        }
        _ => {}
    }
}

/// Recursively traverses a JSON value to collect YouTube videos.
pub(crate) fn extract_videos(val: &Value, out: &mut Vec<SongItem>) {
    match val {
        Value::Object(map) => {
            if let Some(lum) = map.get("lockupViewModel") {
                if let Some(item) = parse_lockup_view_model(lum) {
                    if !out.iter().any(|v| v.video_id == item.video_id) {
                        out.push(item);
                    }
                }
            } else if let Some(vr) = map.get("videoRenderer") {
                if let Some(item) = parse_video_renderer(vr) {
                    if !out.iter().any(|v| v.video_id == item.video_id) {
                        out.push(item);
                    }
                }
            } else if let Some(cvr) = map.get("compactVideoRenderer") {
                if let Some(item) = parse_compact_video_renderer(cvr) {
                    if !out.iter().any(|v| v.video_id == item.video_id) {
                        out.push(item);
                    }
                }
            } else if let Some(gvr) = map.get("gridVideoRenderer") {
                if let Some(item) = parse_grid_video_renderer(gvr) {
                    if !out.iter().any(|v| v.video_id == item.video_id) {
                        out.push(item);
                    }
                }
            } else if let Some(pvr) = map.get("playlistVideoRenderer") {
                if let Some(item) = parse_video_renderer(pvr) {
                    if !out.iter().any(|v| v.video_id == item.video_id) {
                        out.push(item);
                    }
                }
            }

            for v in map.values() {
                extract_videos(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                extract_videos(v, out);
            }
        }
        _ => {}
    }
}

/// Parses a modern `lockupViewModel` into a standard `SongItem`.
fn parse_lockup_view_model(lum: &Value) -> Option<SongItem> {
    let video_id = lum.get("contentId")?.as_str()?.to_string();
    if video_id.is_empty() {
        return None;
    }

    let meta = lum.get("metadata")?.get("lockupMetadataViewModel")?;
    let title = meta.get("title")?.get("content")?.as_str()?.to_string();

    let mut channel = String::new();
    let mut badge = String::new();

    let rows_opt = meta
        .get("metadata")
        .and_then(|m| m.get("contentMetadataViewModel"))
        .and_then(|cm| cm.get("metadataRows"))
        .or_else(|| meta.get("metadataRows"))
        .and_then(|r| r.as_array());

    if let Some(rows) = rows_opt {
        for row in rows {
            if let Some(parts) = row.get("metadataParts").and_then(|p| p.as_array()) {
                let texts: Vec<String> = parts
                    .iter()
                    .filter_map(|p| {
                        p.get("text")
                            .and_then(extract_text_val)
                            .or_else(|| extract_text_val(p))
                    })
                    .collect();

                if texts.is_empty() {
                    continue;
                }

                let is_stats = texts.iter().any(|t| {
                    let low = t.to_lowercase();
                    low.contains("view")
                        || low.contains("ago")
                        || low.contains("watching")
                        || low.contains("streamed")
                        || low.contains("subscriber")
                        || low.contains("ditonton")
                        || low.contains("yang lalu")
                });

                if is_stats {
                    if badge.is_empty() {
                        badge = texts.join(" • ");
                    }
                } else if channel.is_empty() {
                    channel = clean_channel_name(&texts.join(" "));
                }
            }
        }
    }

    // Fallback 1: decoratedAvatarViewModel a11yLabel ("Go to channel X")
    if channel.is_empty() {
        if let Some(lbl) = meta
            .get("image")
            .and_then(|img| img.get("decoratedAvatarViewModel"))
            .and_then(|davm| davm.get("a11yLabel").and_then(|l| l.as_str()))
        {
            channel = clean_channel_name(lbl);
        }
    }

    // Fallback 2: nested avatarViewModel accessibilityData
    if channel.is_empty() {
        if let Some(lbl) = meta
            .get("image")
            .and_then(|img| img.get("decoratedAvatarViewModel"))
            .and_then(|davm| davm.get("avatar"))
            .and_then(|av| av.get("avatarViewModel"))
            .and_then(|avm| avm.get("accessibility"))
            .and_then(|acc| acc.get("accessibilityData"))
            .and_then(|ad| ad.get("label"))
            .and_then(|l| l.as_str())
        {
            channel = clean_channel_name(lbl);
        }
    }

    // Fallback 3: canonicalBaseUrl from browseEndpoint (e.g. "/@samandcolby")
    if channel.is_empty() {
        if let Some(url) = meta
            .get("image")
            .and_then(|img| img.get("decoratedAvatarViewModel"))
            .and_then(|davm| davm.get("rendererContext"))
            .and_then(|rc| rc.get("commandContext"))
            .and_then(|cc| cc.get("onTap"))
            .and_then(|ot| ot.get("innertubeCommand"))
            .and_then(|cmd| cmd.get("browseEndpoint"))
            .and_then(|be| be.get("canonicalBaseUrl"))
            .and_then(|u| u.as_str())
        {
            let cleaned = url.trim_start_matches("/@").trim_start_matches('/');
            if !cleaned.is_empty() {
                channel = cleaned.to_string();
            }
        }
    }

    if channel.is_empty() {
        channel = "YouTube Creator".to_string();
    }

    // Extract duration from thumbnailViewModel overlays
    let mut duration = None;
    if let Some(ci) = lum.get("contentImage") {
        if let Some(overlays) = ci
            .get("thumbnailViewModel")
            .and_then(|tvm| tvm.get("overlays"))
        {
            duration = extract_duration_from_overlays(overlays);
        }
    }
    if duration.is_none() {
        if let Some(overlays) = lum.get("overlays") {
            duration = extract_duration_from_overlays(overlays);
        }
    }

    let mut thumbnail = None;
    if let Some(sources) = lum
        .get("contentImage")
        .and_then(|ci| ci.get("thumbnailViewModel"))
        .and_then(|tvm| tvm.get("image"))
        .and_then(|img| img.get("sources"))
        .and_then(|s| s.as_array())
    {
        if let Some(last) = sources.last() {
            thumbnail = last.get("url").and_then(|u| u.as_str()).map(clean_image_url);
        }
    }

    Some(SongItem {
        video_id,
        title,
        artists: channel,
        artist_id: None,
        artist_runs: Vec::new(),
        album: if badge.is_empty() { None } else { Some(badge) },
        album_id: None,
        duration,
        play_count: None,
        thumbnail,
        set_video_id: None,
        added_by: None,
        added_by_avatars: Vec::new(),
        rating: None,
        queued_by: None,
        queued: false,
        queued_end: false,
        queued_from: None,
        autoplay: false,
        is_video: true,
        counterpart: None,
        is_upload: false,
        explicit: false,
        library: None,
    })
}

/// Parses a classic `videoRenderer` into a standard `SongItem`.
fn parse_video_renderer(vr: &Value) -> Option<SongItem> {
    let video_id = vr.get("videoId")?.as_str()?.to_string();
    if video_id.is_empty() {
        return None;
    }

    let title = vr
        .get("title")
        .and_then(extract_text_val)
        .unwrap_or_default();

    let channel = vr
        .get("ownerText")
        .or_else(|| vr.get("shortBylineText"))
        .or_else(|| vr.get("longBylineText"))
        .and_then(extract_text_val)
        .map(|s| clean_channel_name(&s))
        .unwrap_or_else(|| "YouTube Creator".to_string());

    let views = vr
        .get("viewCountText")
        .and_then(|v| v.get("simpleText").or_else(|| v.get("runs").and_then(|r| r.get(0)).and_then(|f| f.get("text"))))
        .and_then(|s| s.as_str())
        .unwrap_or_default();

    let time_ago = vr
        .get("publishedTimeText")
        .and_then(|p| p.get("simpleText").or_else(|| p.get("runs").and_then(|r| r.get(0)).and_then(|f| f.get("text"))))
        .and_then(|s| s.as_str())
        .unwrap_or_default();

    let badge = format!("{views} • {time_ago}").trim_matches(&[' ', '•'][..]).to_string();

    let mut duration = vr
        .get("lengthText")
        .and_then(|l| l.get("simpleText").or_else(|| l.get("runs").and_then(|r| r.get(0)).and_then(|f| f.get("text"))))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    if duration.is_none() {
        if let Some(overlays) = vr.get("thumbnailOverlays") {
            duration = extract_duration_from_overlays(overlays);
        }
    }

    let thumbnail = vr
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.last())
        .and_then(|u| u.get("url"))
        .and_then(|u| u.as_str())
        .map(clean_image_url);

    Some(SongItem {
        video_id,
        title,
        artists: channel,
        artist_id: None,
        artist_runs: Vec::new(),
        album: if badge.is_empty() { None } else { Some(badge) },
        album_id: None,
        duration,
        play_count: None,
        thumbnail,
        set_video_id: None,
        added_by: None,
        added_by_avatars: Vec::new(),
        rating: None,
        queued_by: None,
        queued: false,
        queued_end: false,
        queued_from: None,
        autoplay: false,
        is_video: true,
        counterpart: None,
        is_upload: false,
        explicit: false,
        library: None,
    })
}

/// Parses a `compactVideoRenderer` into a standard `SongItem`.
fn parse_compact_video_renderer(cvr: &Value) -> Option<SongItem> {
    let video_id = cvr.get("videoId")?.as_str()?.to_string();
    if video_id.is_empty() {
        return None;
    }

    let title = cvr
        .get("title")
        .and_then(extract_text_val)
        .unwrap_or_default();

    let channel = cvr
        .get("shortBylineText")
        .or_else(|| cvr.get("longBylineText"))
        .or_else(|| cvr.get("ownerText"))
        .and_then(extract_text_val)
        .map(|s| clean_channel_name(&s))
        .unwrap_or_else(|| "YouTube Creator".to_string());

    let views = cvr
        .get("viewCountText")
        .and_then(extract_text_val)
        .unwrap_or_default();

    let time_ago = cvr
        .get("publishedTimeText")
        .and_then(extract_text_val)
        .unwrap_or_default();

    let badge = format!("{views} • {time_ago}").trim_matches(&[' ', '•'][..]).to_string();

    let mut duration = cvr
        .get("lengthText")
        .and_then(extract_text_val);

    if duration.is_none() {
        if let Some(overlays) = cvr.get("thumbnailOverlays") {
            duration = extract_duration_from_overlays(overlays);
        }
    }

    let thumbnail = cvr
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.last())
        .and_then(|u| u.get("url"))
        .and_then(|u| u.as_str())
        .map(clean_image_url);

    Some(SongItem {
        video_id,
        title,
        artists: channel,
        artist_id: None,
        artist_runs: Vec::new(),
        album: if badge.is_empty() { None } else { Some(badge) },
        album_id: None,
        duration,
        play_count: None,
        thumbnail,
        set_video_id: None,
        added_by: None,
        added_by_avatars: Vec::new(),
        rating: None,
        queued_by: None,
        queued: false,
        queued_end: false,
        queued_from: None,
        autoplay: false,
        is_video: true,
        counterpart: None,
        is_upload: false,
        explicit: false,
        library: None,
    })
}

/// Parses a `gridVideoRenderer` (common on channel video uploads) into a standard `SongItem`.
fn parse_grid_video_renderer(gvr: &Value) -> Option<SongItem> {
    let video_id = gvr.get("videoId")?.as_str()?.to_string();
    if video_id.is_empty() {
        return None;
    }

    let title = gvr
        .get("title")
        .and_then(extract_text_val)
        .unwrap_or_default();

    let channel = gvr
        .get("shortBylineText")
        .or_else(|| gvr.get("ownerText"))
        .and_then(extract_text_val)
        .map(|s| clean_channel_name(&s))
        .unwrap_or_else(|| "YouTube Creator".to_string());

    let views = gvr
        .get("viewCountText")
        .and_then(|v| v.get("simpleText").or_else(|| v.get("runs").and_then(|r| r.get(0)).and_then(|f| f.get("text"))))
        .and_then(|s| s.as_str())
        .unwrap_or_default();

    let time_ago = gvr
        .get("publishedTimeText")
        .and_then(|p| p.get("simpleText").or_else(|| p.get("runs").and_then(|r| r.get(0)).and_then(|f| f.get("text"))))
        .and_then(|s| s.as_str())
        .unwrap_or_default();

    let badge = format!("{views} • {time_ago}").trim_matches(&[' ', '•'][..]).to_string();

    let mut duration = gvr
        .get("lengthText")
        .and_then(|l| l.get("simpleText").or_else(|| l.get("runs").and_then(|r| r.get(0)).and_then(|f| f.get("text"))))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    if duration.is_none() {
        if let Some(overlays) = gvr.get("thumbnailOverlays") {
            duration = extract_duration_from_overlays(overlays);
        }
    }

    let thumbnail = gvr
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.last())
        .and_then(|u| u.get("url"))
        .and_then(|u| u.as_str())
        .map(clean_image_url);

    Some(SongItem {
        video_id,
        title,
        artists: channel,
        artist_id: None,
        artist_runs: Vec::new(),
        album: if badge.is_empty() { None } else { Some(badge) },
        album_id: None,
        duration,
        play_count: None,
        thumbnail,
        set_video_id: None,
        added_by: None,
        added_by_avatars: Vec::new(),
        rating: None,
        queued_by: None,
        queued: false,
        queued_end: false,
        queued_from: None,
        autoplay: false,
        is_video: true,
        counterpart: None,
        is_upload: false,
        explicit: false,
        library: None,
    })
}

/// Cleans protocol-relative URLs.
fn clean_image_url(url: &str) -> String {
    if url.starts_with("//") {
        format!("https:{url}")
    } else {
        url.to_string()
    }
}

/// Fetches the Subscriptions feed (`FEsubscriptions`) for the signed-in user.
pub async fn get_youtube_subscriptions(db: &Db) -> Result<Vec<SongItem>, String> {
    let cookie = db
        .get_setting("session_cookie")
        .unwrap_or_default()
        .trim()
        .to_string();

    if cookie.is_empty() {
        return Err("Anda belum login dengan akun Google. Silakan login terlebih dahulu untuk melihat video subscriptions.".to_string());
    }

    let visitor_data = db.get_setting("visitor_data").filter(|s| !s.is_empty());
    let data = post_browse(&cookie, "FEsubscriptions", None, visitor_data.as_deref()).await?;
    let mut videos = Vec::new();
    extract_videos(&data, &mut videos);
    Ok(videos)
}

/// Fetches the list of channels the user subscribed to (`FEchannels`).
pub async fn get_subscribed_channels(db: &Db) -> Result<Vec<SubscribedChannel>, String> {
    let cookie = db
        .get_setting("session_cookie")
        .unwrap_or_default()
        .trim()
        .to_string();

    if cookie.is_empty() {
        return Err("Anda belum login dengan akun Google. Silakan login terlebih dahulu untuk melihat daftar channel.".to_string());
    }

    let visitor_data = db.get_setting("visitor_data").filter(|s| !s.is_empty());
    let data = post_browse(&cookie, "FEchannels", None, visitor_data.as_deref()).await?;
    let mut channels = Vec::new();

    fn extract_channels(val: &Value, out: &mut Vec<SubscribedChannel>) {
        match val {
            Value::Object(map) => {
                if let Some(cr) = map.get("channelRenderer") {
                    if let Some(id) = cr.get("channelId").and_then(|i| i.as_str()) {
                        let title = cr
                            .get("title")
                            .and_then(|t| {
                                t.get("simpleText")
                                    .or_else(|| t.get("runs").and_then(|r| r.get(0)).and_then(|f| f.get("text")))
                            })
                            .and_then(|s| s.as_str())
                            .unwrap_or_default()
                            .to_string();

                        let sub_count = cr
                            .get("videoCountText")
                            .or_else(|| cr.get("subscriberCountText"))
                            .and_then(|s| s.get("simpleText"))
                            .and_then(|s| s.as_str())
                            .unwrap_or_default()
                            .to_string();

                        let avatar = cr
                            .get("thumbnail")
                            .and_then(|t| t.get("thumbnails"))
                            .and_then(|arr| arr.as_array())
                            .and_then(|arr| arr.last())
                            .and_then(|u| u.get("url"))
                            .and_then(|u| u.as_str())
                            .map(clean_image_url)
                            .unwrap_or_default();

                        let desc = cr
                            .get("descriptionSnippet")
                            .and_then(|d| d.get("runs"))
                            .and_then(|r| r.get(0))
                            .and_then(|f| f.get("text"))
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());

                        if !id.is_empty() && !title.is_empty() && !out.iter().any(|c| c.id == id) {
                            out.push(SubscribedChannel {
                                id: id.to_string(),
                                title,
                                subscriber_count: sub_count,
                                video_count: None,
                                avatar_url: avatar,
                                description: desc,
                            });
                        }
                    }
                }
                for v in map.values() {
                    extract_channels(v, out);
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    extract_channels(v, out);
                }
            }
            _ => {}
        }
    }

    extract_channels(&data, &mut channels);
    Ok(channels)
}

/// Fetches the latest videos uploaded by a specific YouTube channel.
pub async fn get_channel_videos(db: &Db, channel_id: &str) -> Result<Vec<SongItem>, String> {
    let cookie = db
        .get_setting("session_cookie")
        .unwrap_or_default()
        .trim()
        .to_string();

    let visitor_data = db.get_setting("visitor_data").filter(|s| !s.is_empty());
    // "Egl2aWRlb3PyBgQKAjoA" is the protobuf parameter for the channel's "Videos" tab
    let data = post_browse(&cookie, channel_id, Some("Egl2aWRlb3PyBgQKAjoA"), visitor_data.as_deref()).await?;
    let mut videos = Vec::new();
    extract_videos(&data, &mut videos);
    Ok(videos)
}

/// A feed page containing personalized recommendations or filtered category videos alongside dynamic YouTube chips.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoFeedPage {
    pub chips: Vec<innertube::HomeChip>,
    pub videos: Vec<SongItem>,
    pub continuation: Option<String>,
}

/// Fetches personalized home recommendations (`FEwhat_to_watch`) with dynamic YouTube category chips,
/// or loads a specific chip filter using its continuation token.
pub async fn get_youtube_video_feed(
    db: &Db,
    token: Option<&str>,
    it: Option<&InnerTube>,
    client: Option<&YouTubeClient>,
) -> Result<VideoFeedPage, String> {
    let cookie = db
        .get_setting("session_cookie")
        .unwrap_or_default()
        .trim()
        .to_string();

    let visitor_data = db.get_setting("visitor_data").filter(|s| !s.is_empty());

    let mut videos = Vec::new();
    let mut chips = Vec::new();

    if let Some(t) = token {
        let trimmed_token = t.trim();
        if !trimmed_token.is_empty() {
            let client = get_client();
            let headers = build_yt_headers(&cookie);
            let mut client_obj = serde_json::json!({
                "clientName": "WEB",
                "clientVersion": "2.20240101.00.00",
                "hl": "en",
                "gl": "US"
            });
            if let Some(vd) = &visitor_data {
                client_obj["visitorData"] = serde_json::Value::String(vd.clone());
            }
            let body = serde_json::json!({
                "context": {
                    "client": client_obj
                },
                "continuation": trimmed_token
            });

            if let Ok(resp) = client.post(YT_BROWSE_URL).headers(headers).json(&body).send().await {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        extract_videos(&json, &mut videos);
                    }
                }
            }

            return Ok(VideoFeedPage {
                chips: Vec::new(),
                videos,
                continuation: None,
            });
        }
    }

    if let Ok(data) = post_browse(&cookie, "FEwhat_to_watch", None, visitor_data.as_deref()).await {
        extract_chips(&data, &mut chips);
        extract_videos(&data, &mut videos);
    }

    // Graceful fallback if anonymous or YouTube returned empty for FEwhat_to_watch
    if videos.is_empty() {
        if let (Some(it), Some(client)) = (it, client) {
            if let Ok(trending) = get_trending_videos("featured", it, client).await {
                videos = trending;
            }
        }
    }

    Ok(VideoFeedPage {
        chips,
        videos,
        continuation: None,
    })
}

/// Fetches personalized home recommendations (`FEwhat_to_watch`).
/// If signed in with a Google account, pulls genuine YouTube homepage recommendations.
/// If anonymous or empty, falls back gracefully to curated high-interest video topics.
pub async fn get_youtube_recommendations(
    db: &Db,
    it: Option<&InnerTube>,
    client: Option<&YouTubeClient>,
) -> Result<Vec<SongItem>, String> {
    let feed = get_youtube_video_feed(db, None, it, client).await?;
    Ok(feed.videos)
}

/// Fetches curated non-music trending videos by category.
pub async fn get_trending_videos(
    category: &str,
    it: &InnerTube,
    client: &YouTubeClient,
) -> Result<Vec<SongItem>, String> {
    let query = match category {
        "ai" => "ai artificial intelligence llm agents machine learning -music -song",
        "tech" => "technology tech review engineering future tech gadgets -music -song",
        "science" => "science discovery physics biology ted ed veritasium kurzgesagt -music -song",
        "podcasts" => "english podcast conversation interview full episode -music -song",
        "coding" => "programming software architecture coding live development -music -song",
        _ => "science technology artificial intelligence deep dive documentary -music -song",
    };

    let search_res = it
        .search_videos(client, query)
        .await
        .map_err(|e| format!("Pencarian video gagal: {e}"))?;

    // Filter out any accidentally returned music tracks or empty items
    let filtered: Vec<SongItem> = search_res
        .items
        .into_iter()
        .filter(|v| {
            let title = v.title.to_lowercase();
            !title.contains("official music video")
                && !title.contains("official audio")
                && !title.contains("lyric video")
        })
        .collect();

    Ok(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_what_to_watch() {
        let db_path = std::path::PathBuf::from("/home/butterfly_student/.local/share/com.limusic.desktop/limusic.sqlite");
        if db_path.exists() {
            let (db, _) = crate::db::Db::open_or_quarantine(&db_path).unwrap();
            let feed = get_youtube_video_feed(&db, None, None, None).await;
            if let Ok(page) = feed {
                assert!(!page.videos.is_empty(), "Videos should not be empty");
            }
        }
    }
}
