//! Movie scene shadowing module for Immersio.
//! Provides curated movie clips (<10 minutes) with Closed Captions for sentence-by-sentence shadowing.

use innertube::{InnerTube, SongItem, YouTubeClient};

/// Helper to verify a video duration is strictly under 10 minutes (<= 10:30).
pub fn is_under_10_minutes(dur_str: Option<&str>) -> bool {
    let Some(d) = dur_str else { return true };
    let parts: Vec<&str> = d.trim().split(':').collect();
    match parts.len() {
        1 => true, // seconds only (< 60s)
        2 => {
            // mm:ss
            if let Ok(minutes) = parts[0].parse::<u32>() {
                minutes <= 10
            } else {
                true
            }
        }
        _ => false, // hh:mm:ss -> > 1 hour
    }
}

fn make_preset(video_id: &str, title: &str, channel: &str, cefr_info: &str, duration: &str) -> SongItem {
    SongItem {
        video_id: video_id.into(),
        title: title.into(),
        artists: channel.into(),
        album: Some(cefr_info.into()),
        duration: Some(duration.into()),
        thumbnail: Some(format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg")),
        ..Default::default()
    }
}

/// Curated starter collection of iconic movie and TV scenes (<10 mins) with CEFR tags.
pub fn starter_scenes() -> Vec<SongItem> {
    vec![
        make_preset("qM-gZintNWc", "Good Will Hunting - Park Bench Scene (Sean & Will)", "Miramax / Movieclips", "CEFR B2 • Drama", "4:36"),
        make_preset("ubTJI_UphPk", "BBC Drama Shorts - Vincent and the Doctor (Museum Scene)", "BBC Drama Shorts", "CEFR B1-B2 • British Drama", "3:42"),
        make_preset("s5vT8Q2JjZk", "The Social Network - Opening Scene (Mark & Erica)", "Sony Pictures / Movieclips", "CEFR B2 • Rapid Dialogue", "4:52"),
        make_preset("veYR3ZnvY4w", "Dead Poets Society - Carpe Diem (Make Lives Extraordinary)", "Touchstone / Movieclips", "CEFR C1 • Inspirational Rhetoric", "4:48"),
        make_preset("1XMyx-0j4i0", "Inception - Cafe Dream Scene (Cobb Explains Architecture)", "Warner Bros. / Movieclips", "CEFR B2 • Sci-Fi Dialogue", "3:54"),
        make_preset("nnp2V6aP8vI", "The Dark Knight - Police Interrogation Room (Batman & Joker)", "Warner Bros. / Movieclips", "CEFR B2 • Intense Monologue", "4:18"),
        make_preset("g79FhH0p49E", "Friends - Joey & Chandler London Trip Scene", "Learn English With TV Series", "CEFR A2-B1 • Sitcom Conversation", "3:22"),
        make_preset("WaaANll8h18", "The Office - Jim Impersonates Dwight (Bears, Beets)", "Peacock / The Office", "CEFR A2-B1 • Workplace Humor", "2:14"),
        make_preset("7uQn2i8nE0I", "BBC Drama Shorts - Sherlock First Deduction Scene", "BBC Drama Shorts", "CEFR B2 • Fast British English", "4:10"),
        make_preset("q4pDUxnj52U", "TED-Ed - The Myth of Sisyphus (Clear Articulation)", "TED-Ed Shorts", "CEFR B1 • Narrative Short", "4:57"),
    ]
}

/// Fetch movie and drama scenes (<10 mins) filtered for English shadowing.
pub async fn get_shadowing_scenes(
    category: &str,
    query: Option<&str>,
    it: &InnerTube,
    client: &YouTubeClient,
) -> Result<Vec<SongItem>, String> {
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
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && !title.contains("official audio")
                        && !title.contains("lyric video")
                        && !title.contains("full movie")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();
            return Ok(filtered);
        }
    }

    // Category routing
    match category {
        "starter" => Ok(starter_scenes()),
        "bbc" | "bbc_drama" => {
            let search_query = "BBC Drama Shorts scene dialogue -music -song";
            let search_res = it
                .search_videos(client, search_query)
                .await
                .map_err(|e| format!("Gagal memuat BBC Drama Shorts: {e}"))?;

            let mut filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && !title.contains("official audio")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();

            // Include starter BBC presets at top if available
            let bbc_starters: Vec<SongItem> = starter_scenes()
                .into_iter()
                .filter(|s| s.artists.contains("BBC"))
                .collect();

            for s in bbc_starters.into_iter().rev() {
                if !filtered.iter().any(|f| f.video_id == s.video_id) {
                    filtered.insert(0, s);
                }
            }
            Ok(filtered)
        }
        "movieclips" => {
            let search_query = "Rotten Tomatoes Movieclips iconic movie scene -music -song";
            let search_res = it
                .search_videos(client, search_query)
                .await
                .map_err(|e| format!("Gagal memuat Movieclips: {e}"))?;

            let filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();
            Ok(filtered)
        }
        "binge" => {
            let search_query = "Binge Society movie scene dialogue -music -song";
            let search_res = it
                .search_videos(client, search_query)
                .await
                .map_err(|e| format!("Gagal memuat Binge Society: {e}"))?;

            let filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();
            Ok(filtered)
        }
        "scenecity" => {
            let search_query = "Scene City dramatic movie scene dialogue -music -song";
            let search_res = it
                .search_videos(client, search_query)
                .await
                .map_err(|e| format!("Gagal memuat Scene City: {e}"))?;

            let filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();
            Ok(filtered)
        }
        "series" => {
            let search_query = "Learn English with TV series scene dialogue -music -song";
            let search_res = it
                .search_videos(client, search_query)
                .await
                .map_err(|e| format!("Gagal memuat TV Series: {e}"))?;

            let filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();
            Ok(filtered)
        }
        "omeleto" => {
            let search_query = "Omeleto short film drama dialogue -music -song";
            let search_res = it
                .search_videos(client, search_query)
                .await
                .map_err(|e| format!("Gagal memuat Omeleto: {e}"))?;

            let filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();
            Ok(filtered)
        }
        "ted" => {
            let search_query = "TED-Ed animated story short english -music -song";
            let search_res = it
                .search_videos(client, search_query)
                .await
                .map_err(|e| format!("Gagal memuat TED-Ed: {e}"))?;

            let filtered: Vec<SongItem> = search_res
                .items
                .into_iter()
                .filter(|v| {
                    let title = v.title.to_lowercase();
                    !title.contains("official music video")
                        && is_under_10_minutes(v.duration.as_deref())
                })
                .collect();
            Ok(filtered)
        }
        _ => {
            // "all" - Start with handpicked starter presets, then append live search results
            let mut results = starter_scenes();
            let search_query = "movie scene dialogue clip english -music -song";
            if let Ok(search_res) = it.search_videos(client, search_query).await {
                for item in search_res.items {
                    let title = item.title.to_lowercase();
                    if !title.contains("official music video")
                        && !title.contains("official audio")
                        && is_under_10_minutes(item.duration.as_deref())
                        && !results.iter().any(|r| r.video_id == item.video_id)
                    {
                        results.push(item);
                    }
                }
            }
            Ok(results)
        }
    }
}
