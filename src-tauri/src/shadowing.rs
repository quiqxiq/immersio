//! Movie scene shadowing module for Immersio.
//! Provides curated dialogue scenes strictly between 2-15 minutes for English shadowing.

use innertube::{InnerTube, SongItem, YouTubeClient};
use serde_json::Value;

use crate::db::Db;

/// Helper to verify a video duration is strictly between 2 and 15 minutes (120s <= duration <= 900s).
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

fn make_preset(video_id: &str, title: &str, channel: &str, duration: &str) -> SongItem {
    SongItem {
        video_id: video_id.into(),
        title: title.into(),
        artists: channel.into(),
        album: None, // No CEFR tag
        duration: Some(duration.into()),
        thumbnail: Some(format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg")),
        is_video: true,
        ..Default::default()
    }
}

/// Curated starter collection of iconic dialogue scenes strictly between 2-15 minutes (without CEFR tags).
pub fn starter_scenes() -> Vec<SongItem> {
    vec![
        make_preset("qM-gZintNWc", "Good Will Hunting - Park Bench Scene (Sean & Will)", "Miramax / Movieclips", "4:36"),
        make_preset("s5vT8Q2JjZk", "The Social Network - Opening Scene (Mark & Erica)", "Sony Pictures / Movieclips", "4:52"),
        make_preset("veYR3ZnvY4w", "Dead Poets Society - Carpe Diem (Make Lives Extraordinary)", "Touchstone / Movieclips", "4:48"),
        make_preset("1XMyx-0j4i0", "Inception - Cafe Dream Scene (Cobb Explains Architecture)", "Warner Bros. / Movieclips", "3:54"),
        make_preset("nnp2V6aP8vI", "The Dark Knight - Police Interrogation Room (Batman & Joker)", "Warner Bros. / Movieclips", "4:18"),
        make_preset("g79FhH0p49E", "Friends - Joey & Chandler London Trip Scene", "Learn English With TV Series", "3:22"),
        make_preset("WaaANll8h18", "The Office - Jim Impersonates Dwight (Bears, Beets)", "Peacock / The Office", "2:14"),
        make_preset("u0MqWulYGDQ", "King Kong's Brutal Battle With The T-Rex (Full Scene)", "Critic Picks", "5:08"),
        make_preset("p9DQSnik1mw", "Al Pacino's Iconic Final Speech (Full Scene)", "Critic Picks", "6:15"),
        make_preset("T4qKtXIKo2w", "Barry Seal Meets CIA Agent Monty Shafer | American Made", "Critic Picks", "5:08"),
    ]
}

/// Recursively find Popular continuation token from YouTube browse response
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

/// Filter video: strictly 2-15 minutes, non-music, clean album (no CEFR)
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

/// Fetches 20 most popular and 10 newest videos strictly between 2-15 minutes for a channel.
pub async fn fetch_channel_scenes(
    cookie: &str,
    channel_id: &str,
    channel_name: &str,
) -> Result<Vec<SongItem>, String> {
    // 1. Fetch channel's Videos tab (returns Latest videos by default, and Popular token in chipBar)
    let data = crate::videos::post_browse(cookie, channel_id, Some("EgZ2aWRlb3PyBgQKAjoA")).await?;

    let mut raw_latest = Vec::new();
    crate::videos::extract_videos(&data, &mut raw_latest);

    let pop_token = find_popular_token(&data);
    let mut raw_popular = Vec::new();
    if let Some(token) = pop_token {
        if let Ok(pop_data) = crate::videos::post_browse_continuation(cookie, &token).await {
            crate::videos::extract_videos(&pop_data, &mut raw_popular);
        }
    }

    // Filter and take up to 20 popular
    let mut popular_videos: Vec<SongItem> = raw_popular
        .into_iter()
        .filter_map(|v| filter_scene_item(v, channel_name))
        .collect();
    popular_videos.truncate(20);

    // Filter and take up to 10 latest
    let mut latest_videos: Vec<SongItem> = raw_latest
        .into_iter()
        .filter_map(|v| filter_scene_item(v, channel_name))
        .collect();
    latest_videos.truncate(10);

    // Combine: 20 popular first, then 10 latest (dedup by video_id)
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
    match category {
        "starter" => Ok(starter_scenes()),
        "criticpicks" => {
            // @CriticPicks
            fetch_channel_scenes(&cookie, "UCxwQGJztPxV3jqqxp8Cv43g", "Critic Picks").await
        }
        "clipempire" => {
            // @ClipEmpireChannel
            fetch_channel_scenes(&cookie, "UCtMEpYpBm3YVo5ijQzUZdLg", "Clip Empire").await
        }
        "dollartheater" => {
            // @TheDollarTheater
            fetch_channel_scenes(&cookie, "UCFR6sruqEq52xEqjE84tq4A", "The Dollar Theater").await
        }
        "binge" => {
            // @bingesociety
            fetch_channel_scenes(&cookie, "UCOo_v3eVbfET7_zi2KLOP9g", "Binge Society").await
        }
        "firstslasts" => {
            // @firstsandlastsofficial
            fetch_channel_scenes(&cookie, "UCSxaMpz_ZG05fkp5OFTUO1w", "Firsts and Lasts").await
        }
        "movieclips" => {
            // Rotten Tomatoes Movieclips
            fetch_channel_scenes(&cookie, "UC3gNmTGu-TTbFPpfSs5kNkg", "Movieclips").await
        }
        "scenecity" => {
            // Universal Pictures Scene City
            fetch_channel_scenes(&cookie, "UC5N2asRYkdATmJ9dtr1fO1w", "Scene City").await
        }
        "omeleto" => {
            // Omeleto Short Films
            fetch_channel_scenes(&cookie, "UCTMt7iMWa7jy0fNXIktwyLA", "Omeleto").await
        }
        "series" => {
            // Learn English With TV Series
            fetch_channel_scenes(&cookie, "UCKgpamMlm872zkGDcBJHYDg", "Learn English With TV Series").await
        }
        _ => {
            // "all" - Combine starter presets with top popular scenes from Critic Picks, Binge Society, and Movieclips
            let mut results = starter_scenes();
            if let Ok(critic_scenes) = fetch_channel_scenes(&cookie, "UCxwQGJztPxV3jqqxp8Cv43g", "Critic Picks").await {
                for item in critic_scenes.into_iter().take(10) {
                    if !results.iter().any(|r| r.video_id == item.video_id) {
                        results.push(item);
                    }
                }
            }
            if let Ok(binge_scenes) = fetch_channel_scenes(&cookie, "UCOo_v3eVbfET7_zi2KLOP9g", "Binge Society").await {
                for item in binge_scenes.into_iter().take(10) {
                    if !results.iter().any(|r| r.video_id == item.video_id) {
                        results.push(item);
                    }
                }
            }
            Ok(results)
        }
    }
}
