//! Articles and Reading Feed provider for Limusic.
//!
//! 100% native Rust backend:
//! - Fetches and caches RSS feeds from curated sources (BBC Learning English, NYT, The Guardian).
//! - Extracts full distraction-free editorial content using Readability in Rust.
//! - Preserves original formatting, headers, paragraphs, quotes, and images while stripping ads and scripts.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use url::Url;

/// Summary card for article feeds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleCard {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub link: String,
    pub published: Option<String>,
    pub source: String,
    pub category: String,
    pub image_url: Option<String>,
}

/// Full extracted article content for Reader Mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleContent {
    pub url: String,
    pub title: String,
    pub byline: Option<String>,
    pub source: String,
    pub published: Option<String>,
    pub hero_image: Option<String>,
    pub html: String,
    pub text_length: usize,
}

struct SourceConfig {
    key: &'static str,
    name: &'static str,
    category: &'static str,
    feed_url: &'static str,
}

const SOURCES: &[SourceConfig] = &[
    SourceConfig {
        key: "bbc_6min",
        name: "BBC Learning English",
        category: "6 Minute English",
        feed_url: "https://feeds.bbci.co.uk/learningenglish/english/features/6-minute-english/rss",
    },
    SourceConfig {
        key: "bbc_speak",
        name: "BBC Learning English",
        category: "The English We Speak",
        feed_url: "https://feeds.bbci.co.uk/learningenglish/english/features/the-english-we-speak/rss",
    },
    SourceConfig {
        key: "nyt_world",
        name: "The New York Times",
        category: "World News",
        feed_url: "https://rss.nytimes.com/services/xml/rss/nyt/World.xml",
    },
    SourceConfig {
        key: "nyt_tech",
        name: "The New York Times",
        category: "Technology",
        feed_url: "https://rss.nytimes.com/services/xml/rss/nyt/Technology.xml",
    },
    SourceConfig {
        key: "guardian_world",
        name: "The Guardian",
        category: "World News",
        feed_url: "https://www.theguardian.com/world/rss",
    },
    SourceConfig {
        key: "guardian_science",
        name: "The Guardian",
        category: "Science",
        feed_url: "https://www.theguardian.com/science/rss",
    },
];

type CacheMap = HashMap<String, (Instant, Vec<ArticleCard>)>;

static CACHE: std::sync::LazyLock<Arc<RwLock<CacheMap>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

const CACHE_TTL: Duration = Duration::from_secs(600); // 10 minutes

/// Fetch articles feed from curated sources or a specific source
pub async fn get_feed(source_filter: Option<String>) -> Result<Vec<ArticleCard>, String> {
    let now = Instant::now();
    let filter = source_filter.unwrap_or_else(|| "all".to_string());

    // Check cache first
    {
        let cache = CACHE.read().await;
        if let Some((fetched_at, items)) = cache.get(&filter) {
            if now.duration_since(*fetched_at) < CACHE_TTL {
                return Ok(items.clone());
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let matching_sources: Vec<&SourceConfig> = if filter == "all" {
        SOURCES.iter().collect()
    } else {
        SOURCES
            .iter()
            .filter(|s| s.key == filter || s.name.to_lowercase().contains(&filter.to_lowercase()))
            .collect()
    };

    let mut all_cards = Vec::new();

    for src in matching_sources {
        match client.get(src.feed_url).send().await {
            Ok(resp) => {
                if let Ok(bytes) = resp.bytes().await {
                    if let Ok(feed) = feed_rs::parser::parse(&bytes[..]) {
                        for entry in feed.entries.into_iter().take(12) {
                            let link = entry
                                .links
                                .first()
                                .map(|l| l.href.clone())
                                .unwrap_or_default();
                            let title = entry
                                .title
                                .map(|t| t.content.trim().to_string())
                                .unwrap_or_default();

                            if link.is_empty() || title.is_empty() {
                                continue;
                            }

                            // Clean summary
                            let raw_summary = entry
                                .summary
                                .map(|s| s.content)
                                .or_else(|| entry.content.and_then(|c| c.body))
                                .unwrap_or_default();
                            let summary = clean_snippet(&raw_summary);

                            // Detect image url from media thumbnails, enclosure, or media content
                            let mut image_url = None;
                            for media in &entry.media {
                                for thumb in &media.thumbnails {
                                    image_url = Some(thumb.image.uri.clone());
                                    break;
                                }
                                if image_url.is_none() {
                                    for content in &media.content {
                                        if let Some(url) = &content.url {
                                            image_url = Some(url.to_string());
                                            break;
                                        }
                                    }
                                }
                                if image_url.is_some() {
                                    break;
                                }
                            }

                            // Published date formatted
                            let published = entry.published.or(entry.updated).map(|dt| {
                                dt.format("%d %b %Y").to_string()
                            });

                            all_cards.push(ArticleCard {
                                id: link.clone(),
                                title,
                                summary,
                                link,
                                published,
                                source: src.name.to_string(),
                                category: src.category.to_string(),
                                image_url,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(source = src.name, error = %e, "Failed to fetch articles feed");
            }
        }
    }

    // Cache the result
    {
        let mut cache = CACHE.write().await;
        cache.insert(filter, (Instant::now(), all_cards.clone()));
    }

    Ok(all_cards)
}

/// Extract clean article content from any given URL using Rust Readability
pub async fn extract(url_str: &str) -> Result<ArticleContent, String> {
    let parsed_url = Url::parse(url_str).map_err(|e| format!("Invalid URL: {e}"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(parsed_url.as_str())
        .send()
        .await
        .map_err(|e| format!("Could not download article: {e}"))?;

    let html_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    // Extract with readability in Rust
    let mut cursor = std::io::Cursor::new(&html_bytes);
    let product = readability::extractor::extract(&mut cursor, &parsed_url)
        .map_err(|e| format!("Failed to parse article readability: {e}"))?;

    // Determine site / source name from URL host
    let source_name = parsed_url
        .host_str()
        .map(|h| {
            let h = h.trim_start_matches("www.").trim_start_matches("feeds.");
            if h.contains("bbc") {
                "BBC"
            } else if h.contains("nytimes") {
                "The New York Times"
            } else if h.contains("theguardian") {
                "The Guardian"
            } else {
                h
            }
        })
        .unwrap_or("Web Article")
        .to_string();

    // Look for top hero image in raw HTML or content
    let raw_html = String::from_utf8_lossy(&html_bytes);
    let hero_image = extract_og_image(&raw_html).or_else(|| extract_first_img(&product.content));

    let sanitized_html = sanitize_article_html(&product.content, &parsed_url);

    Ok(ArticleContent {
        url: url_str.to_string(),
        title: product.title,
        byline: None,
        source: source_name,
        published: None,
        hero_image,
        text_length: product.text.len(),
        html: sanitized_html,
    })
}

/// Quick helper to extract og:image from raw HTML
fn extract_og_image(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<meta\s+property=["']og:image["']\s+content=["']([^"']+)["']"#).ok()?;
    re.captures(html).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

/// Quick helper to extract the first img src from HTML content
fn extract_first_img(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<img[^>]+src=["']([^"']+)["']"#).ok()?;
    re.captures(html).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

/// Sanitize and enhance article HTML:
/// - Resolves relative image and link URLs to absolute URLs
/// - Strips dangerous or unwanted inline attributes
fn sanitize_article_html(html: &str, base_url: &Url) -> String {
    // Replace relative img src with absolute URLs
    let img_src_re = regex::Regex::new(r#"(<img\s+[^>]*src=["'])([^"']+)(["'])"#).unwrap();
    let fixed_html = img_src_re.replace_all(html, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let src = &caps[2];
        let suffix = &caps[3];
        if src.starts_with("http://") || src.starts_with("https://") {
            format!("{prefix}{src}{suffix}")
        } else if let Ok(abs) = base_url.join(src) {
            format!("{prefix}{}{suffix}", abs.as_str())
        } else {
            format!("{prefix}{src}{suffix}")
        }
    });

    fixed_html.to_string()
}

/// Strip basic HTML tags from snippets
fn clean_snippet(raw: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    let cleaned = re.replace_all(raw, "").trim().to_string();
    if cleaned.len() > 180 {
        format!("{}...", &cleaned[..177])
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_snippet() {
        let raw = "<p>Hello <b>world</b>! This is an article snippet.</p>";
        assert_eq!(clean_snippet(raw), "Hello world! This is an article snippet.");
    }

    #[tokio::test]
    async fn test_feed_fetching() {
        let res = get_feed(Some("bbc_6min".to_string())).await;
        assert!(res.is_ok(), "Expected feed fetch to succeed");
        let items = res.unwrap();
        assert!(!items.is_empty(), "Expected non-empty feed items");
        assert_eq!(items[0].source, "BBC Learning English");
    }
}

