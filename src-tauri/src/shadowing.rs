//! Movie scene shadowing module for Immersio.
//! Provides curated dialogue scenes strictly between 2-15 minutes for English shadowing.
//! Each channel fetches 30 most popular + 20 newest videos (with pagination).

use innertube::{InnerTube, SongItem, YouTubeClient};
use serde_json::Value;

use crate::db::Db;

// ── Channel Registry ────────────────────────────────────────────────────────

/// A curated YouTube channel for movie/drama scene shadowing.
struct ShadowChannel {
    slug: &'static str,
    label: &'static str,
    channel_id: &'static str,
}

const CHANNELS: &[ShadowChannel] = &[
    ShadowChannel { slug: "criticpicks",   label: "Critic Picks",      channel_id: "UCxwQGJztPxV3jqqxp8Cv43g" },
    ShadowChannel { slug: "clipempire",    label: "Clip Empire",       channel_id: "UCtMEpYpBm3YVo5ijQzUZdLg" },
    ShadowChannel { slug: "dollartheater", label: "The Dollar Theater", channel_id: "UCFR6sruqEq52xEqjE84tq4A" },
    ShadowChannel { slug: "binge",         label: "Binge Society",     channel_id: "UCOo_v3eVbfET7_zi2KLOP9g" },
    ShadowChannel { slug: "firstslasts",   label: "Firsts and Lasts",  channel_id: "UCSxaMpz_ZG05fkp5OFTUO1w" },
    ShadowChannel { slug: "movieclips",    label: "Movieclips",        channel_id: "UC3gNmTGu-TTbFPpfSs5kNkg" },
    ShadowChannel { slug: "scenecity",     label: "Scene City",        channel_id: "UC5N2asRYkdATmJ9dtr1fO1w" },
    ShadowChannel { slug: "omeleto",       label: "Omeleto",           channel_id: "UCTMt7iMWa7jy0fNXIktwyLA" },
];

// ── Duration Filter ─────────────────────────────────────────────────────────

/// Verify a video duration is strictly between 2 and 15 minutes (120s <= duration <= 900s).
pub fn is_between_2_and_15_minutes(dur_str: Option<&str>) -> bool {
    let Some(d) = dur_str else { return false };
    let d = d.trim();
    if d.is_empty() {
        return false;
    }
    let parts: Vec<&str> = d.split(':').collect();
    let total_secs: u32 = match parts.len() {
        1 => parts[0].parse().unwrap_or(0),
        2 => {
            let m: u32 = parts[0].parse().unwrap_or(0);
            let s: u32 = parts[1].parse().unwrap_or(0);
            m * 60 + s
        }
        3 => {
            let h: u32 = parts[0].parse().unwrap_or(0);
            let m: u32 = parts[1].parse().unwrap_or(0);
            let s: u32 = parts[2].parse().unwrap_or(0);
            h * 3600 + m * 60 + s
        }
        _ => return false,
    };
    total_secs >= 120 && total_secs <= 900
}

// ── Preset Scenes ───────────────────────────────────────────────────────────

fn make_preset(video_id: &str, title: &str, channel: &str, duration: &str) -> SongItem {
    SongItem {
        video_id: video_id.into(),
        title: title.into(),
        artists: channel.into(),
        album: None,
        duration: Some(duration.into()),
        thumbnail: Some(format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg")),
        is_video: true,
        ..Default::default()
    }
}

/// Curated starter collection – best-of-best across all channels with 100% verified live thumbnails.
pub fn starter_scenes() -> Vec<SongItem> {
    vec![
        // Critic Picks
        make_preset("u0MqWulYGDQ", "King Kong's Brutal Battle With The T-Rex (Full Scene)", "Critic Picks", "5:08"),
        make_preset("p9DQSnik1mw", "Al Pacino's Iconic Final Speech (Full Scene) | Scent of a Woman", "Critic Picks", "6:15"),
        make_preset("T4qKtXIKo2w", "Barry Seal Meets CIA Agent Monty Shafer | American Made", "Critic Picks", "5:08"),
        // Movieclips (all verified live 4K/HD with valid thumbnails)
        make_preset("VlSkPA60ujQ", "The Social Network - You're Breaking Up With Me? Scene", "Movieclips", "3:16"),
        make_preset("4Prc1UfuokY", "300 - This Is Sparta! Scene (1/5)", "Movieclips", "2:44"),
        make_preset("7LFtoz9sERo", "Inglourious Basterds - Go Out Speaking the King's Scene", "Movieclips", "4:35"),
        make_preset("6KHyMISpE18", "The Social Network - I'm CEO, Bitch Scene", "Movieclips", "3:30"),
        make_preset("54oDA3ufmj8", "The Dark Knight - Batman vs. Joker Scene", "Movieclips", "2:41"),
        // Clip Empire
        make_preset("WaaANll8h18", "The Office - Jim Impersonates Dwight (Bears, Beets)", "Clip Empire", "2:14"),
        // Binge Society
        make_preset("78oQxv6VN4Q", "Hannibal Lecter is Discovered by FBI Agent Will Graham (Full Scene) | Red Dragon", "Binge Society", "5:30"),
        // The Dollar Theater
        make_preset("J-tJ-9WDLvY", "5 Brutal Cold Gangster Moments", "The Dollar Theater", "8:22"),
        // Scene City
        make_preset("uFc_5PFnBj0", "Babe Ruth Plays His Final Game of Baseball (Full Scene) | The Babe", "Scene City", "4:15"),
        // Omeleto
        make_preset("COjoicAahjo", "The Greatest Sports Scenes Compilation", "Omeleto", "9:44"),
    ]
}

// ── YouTube InnerTube Helpers ───────────────────────────────────────────────

/// Recursively find the "Popular" sort continuation token from a channel Videos tab response.
fn find_popular_token(val: &Value) -> Option<String> {
    match val {
        Value::Object(map) => {
            let is_popular = map
                .get("text")
                .and_then(|t| t.as_str())
                .map(|s| s.eq_ignore_ascii_case("Popular"))
                .unwrap_or(false)
                || map
                    .get("accessibilityLabel")
                    .and_then(|t| t.as_str())
                    .map(|s| s.eq_ignore_ascii_case("Popular"))
                    .unwrap_or(false);

            if is_popular {
                if let Some(tok) = map
                    .get("tapCommand")
                    .and_then(|tc| tc.get("innertubeCommand"))
                    .and_then(|itc| itc.get("continuationCommand"))
                    .and_then(|cc| cc.get("token"))
                    .and_then(|t| t.as_str())
                {
                    return Some(tok.to_string());
                }
            }
            for v in map.values() {
                if let Some(t) = find_popular_token(v) {
                    return Some(t);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                if let Some(t) = find_popular_token(v) {
                    return Some(t);
                }
            }
        }
        _ => {}
    }
    None
}

/// Recursively find the next-page continuation token from a browse/continuation response.
fn find_continuation_token(val: &Value) -> Option<String> {
    match val {
        Value::Object(map) => {
            if let Some(cir) = map.get("continuationItemRenderer") {
                if let Some(tok) = cir
                    .get("continuationEndpoint")
                    .and_then(|ce| ce.get("continuationCommand"))
                    .and_then(|cc| cc.get("token"))
                    .and_then(|t| t.as_str())
                {
                    return Some(tok.to_string());
                }
            }
            for v in map.values() {
                if let Some(t) = find_continuation_token(v) {
                    return Some(t);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                if let Some(t) = find_continuation_token(v) {
                    return Some(t);
                }
            }
        }
        _ => {}
    }
    None
}

/// Filter video: strictly 2-15 minutes, non-music, clean metadata.
fn filter_scene_item(mut item: SongItem, channel_name: &str) -> Option<SongItem> {
    let title_lower = item.title.to_lowercase();
    if title_lower.contains("official music video")
        || title_lower.contains("official audio")
        || title_lower.contains("lyric video")
        || title_lower.contains("full movie")
    {
        return None;
    }

    if !is_between_2_and_15_minutes(item.duration.as_deref()) {
        return None;
    }

    if item.artists.is_empty() || item.artists == "YouTube Creator" {
        item.artists = channel_name.to_string();
    }
    item.album = None;
    item.is_video = true;

    Some(item)
}

/// Fetch videos from a continuation token, collecting items and the next-page token.
async fn fetch_paginated_videos(
    cookie: &str,
    initial_token: &str,
    channel_name: &str,
    target: usize,
    max_pages: usize,
) -> Vec<SongItem> {
    let mut all = Vec::new();
    let mut token = initial_token.to_string();

    for _ in 0..max_pages {
        let data = match crate::videos::post_browse_continuation(cookie, &token).await {
            Ok(d) => d,
            Err(_) => break,
        };

        let mut page_items = Vec::new();
        crate::videos::extract_videos(&data, &mut page_items);

        for item in page_items {
            if let Some(filtered) = filter_scene_item(item, channel_name) {
                if !all.iter().any(|v: &SongItem| v.video_id == filtered.video_id) {
                    all.push(filtered);
                }
            }
        }

        if all.len() >= target {
            break;
        }

        match find_continuation_token(&data) {
            Some(next) => token = next,
            None => break,
        }
    }

    all.truncate(target);
    all
}

/// Fetch latest videos from a channel's Videos tab with pagination.
async fn fetch_latest_videos(
    cookie: &str,
    channel_id: &str,
    channel_name: &str,
    target: usize,
    max_pages: usize,
) -> (Vec<SongItem>, Option<String>) {
    // First page: the Videos tab itself (sorted by Latest by default)
    let data = match crate::videos::post_browse(cookie, channel_id, Some("EgZ2aWRlb3PyBgQKAjoA")).await {
        Ok(d) => d,
        Err(_) => return (Vec::new(), None),
    };

    let pop_token = find_popular_token(&data);

    let mut raw_items = Vec::new();
    crate::videos::extract_videos(&data, &mut raw_items);

    let mut all: Vec<SongItem> = raw_items
        .into_iter()
        .filter_map(|v| filter_scene_item(v, channel_name))
        .collect();

    // Paginate if needed
    if all.len() < target {
        if let Some(cont) = find_continuation_token(&data) {
            let remaining = target - all.len();
            let extra = fetch_paginated_videos(cookie, &cont, channel_name, remaining, max_pages - 1).await;
            for item in extra {
                if !all.iter().any(|v| v.video_id == item.video_id) {
                    all.push(item);
                }
            }
        }
    }

    all.truncate(target);
    (all, pop_token)
}

// ── Main Channel Fetch ──────────────────────────────────────────────────────

/// Fetches 30 most popular + 20 newest videos (2-15 min) for a channel.
/// Uses pagination to ensure we collect enough items from large channels.
pub async fn fetch_channel_scenes(
    cookie: &str,
    channel_id: &str,
    channel_name: &str,
) -> Result<Vec<SongItem>, String> {
    const POPULAR_TARGET: usize = 30;
    const LATEST_TARGET: usize = 20;
    const MAX_PAGES: usize = 3; // Up to 3 pages (~90 raw videos) per sort

    // 1. Fetch Latest videos + extract Popular token
    let (latest_videos, pop_token) = fetch_latest_videos(
        cookie, channel_id, channel_name, LATEST_TARGET, MAX_PAGES,
    ).await;

    // 2. Fetch Popular videos using continuation token
    let popular_videos = if let Some(token) = pop_token {
        fetch_paginated_videos(cookie, &token, channel_name, POPULAR_TARGET, MAX_PAGES).await
    } else {
        Vec::new()
    };

    // 3. Combine: 30 popular first, then 20 latest (dedup by video_id)
    let mut combined = Vec::new();
    for v in popular_videos {
        if !combined.iter().any(|c: &SongItem| c.video_id == v.video_id) {
            combined.push(v);
        }
    }
    for v in latest_videos {
        if !combined.iter().any(|c: &SongItem| c.video_id == v.video_id) {
            combined.push(v);
        }
    }

    Ok(combined)
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Fetch movie and dialogue scenes (strictly 2-15 mins) for English shadowing.
pub async fn get_shadowing_scenes(
    db: &Db,
    category: &str,
    query: Option<&str>,
    it: &InnerTube,
    client: &YouTubeClient,
) -> Result<Vec<SongItem>, String> {
    let cookie = db
        .get_setting("session_cookie")
        .unwrap_or_default()
        .trim()
        .to_string();

    // If custom search query is provided
    if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            let search_query = format!("{trimmed} movie scene clip dialogue -music -song");
            let search_res = it
                .search_videos(client, &search_query)
                .await
                .map_err(|e| format!("Pencarian shadowing gagal: {e}"))?;

            let filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter_map(|v| filter_scene_item(v, "Movie Scene"))
                .collect();
            return Ok(filtered);
        }
    }

    // Category routing
    if category == "starter" {
        return Ok(starter_scenes());
    }

    // Check if category matches a known channel
    for ch in CHANNELS {
        if ch.slug == category {
            return fetch_channel_scenes(&cookie, ch.channel_id, ch.label).await;
        }
    }

    // "all" / default: fetch top scenes from every channel concurrently-ish, merge & dedup
    if category == "all" {
        let mut results = Vec::new();
        // Fetch 8 popular + 4 latest from every channel
        for ch in CHANNELS {
            match fetch_channel_scenes(&cookie, ch.channel_id, ch.label).await {
                Ok(scenes) => {
                    for item in scenes {
                        if !results.iter().any(|r: &SongItem| r.video_id == item.video_id) {
                            results.push(item);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        return Ok(results);
    }

    // Unknown category: starter fallback
    Ok(starter_scenes())
}
