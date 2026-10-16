//! Articles and Reading Feed provider for Immersio.
//!
//! 100% native Rust backend:
//! - 3-Step Local Pipeline:
//!   1. reqwest with modern browser headers
//!   2. Mozilla Readability in native Rust
//!   3. htmd HTML-to-Markdown conversion (for Obsidian/Notion export & AI Tutor)
//! - Hybrid multi-source engine:
//!   * Official REST APIs: Dev.to & Hacker News (Algolia)
//!   * Open RSS feeds: FreeCodeCamp, NPR News, The Guardian, BBC Learning English
//! - Detects paywalls & anti-bot stubs with graceful fallback
//! - Chronological sorting (newest articles first) and 15-minute caching

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
    pub published_at: Option<i64>,
    pub source: String,
    pub category: String,
    pub image_url: Option<String>,
    pub reading_time: Option<u32>,
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
    pub markdown: String,
    pub text_length: usize,
    pub reading_time_minutes: usize,
    pub is_paywall_blocked: bool,
}

struct SourceConfig {
    key: &'static str,
    name: &'static str,
    category: &'static str,
    feed_url: &'static str,
}

const RSS_SOURCES: &[SourceConfig] = &[
    SourceConfig {
        key: "freecodecamp",
        name: "FreeCodeCamp",
        category: "Tutorials & Code",
        feed_url: "https://www.freecodecamp.org/news/rss/",
    },
    SourceConfig {
        key: "npr",
        name: "NPR News",
        category: "World & Culture",
        feed_url: "https://feeds.npr.org/1001/rss.xml",
    },
    SourceConfig {
        key: "guardian",
        name: "The Guardian",
        category: "World News",
        feed_url: "https://www.theguardian.com/world/rss",
    },
    SourceConfig {
        key: "guardian",
        name: "The Guardian",
        category: "Science",
        feed_url: "https://www.theguardian.com/science/rss",
    },
    SourceConfig {
        key: "bbc",
        name: "BBC Learning English",
        category: "6 Minute English",
        feed_url: "https://feeds.bbci.co.uk/learningenglish/english/features/6-minute-english/rss",
    },
    SourceConfig {
        key: "bbc",
        name: "BBC Learning English",
        category: "The English We Speak",
        feed_url: "https://feeds.bbci.co.uk/learningenglish/english/features/the-english-we-speak/rss",
    },
];

type CacheMap = HashMap<String, (Instant, Vec<ArticleCard>)>;

static CACHE: std::sync::LazyLock<Arc<RwLock<CacheMap>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

const CACHE_TTL: Duration = Duration::from_secs(900); // 15 minutes

/// Build an HTTP client with realistic browser headers to prevent anti-bot false alarms
fn build_http_client() -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
            .parse()
            .unwrap(),
    );
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        "en-US,en;q=0.9".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::UPGRADE_INSECURE_REQUESTS,
        "1".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-dest"),
        "document".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-mode"),
        "navigate".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("sec-fetch-site"),
        "none".parse().unwrap(),
    );

    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
        .default_headers(headers)
        .build()
        .map_err(|e| e.to_string())
}

// --- REST API Fetchers: Dev.to & Hacker News ----------------------------------

#[derive(Deserialize)]
struct DevToItem {
    title: String,
    description: Option<String>,
    url: String,
    cover_image: Option<String>,
    social_image: Option<String>,
    readable_publish_date: Option<String>,
    published_at: Option<String>,
    reading_time_minutes: Option<u32>,
    tag_list: Option<Vec<String>>,
}

async fn fetch_devto(client: &reqwest::Client) -> Vec<ArticleCard> {
    let url = "https://dev.to/api/articles?per_page=20&top=1";
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to fetch Dev.to articles: {e}");
            return Vec::new();
        }
    };

    let items: Vec<DevToItem> = match resp.json().await {
        Ok(it) => it,
        Err(e) => {
            tracing::warn!("Failed to parse Dev.to articles json: {e}");
            return Vec::new();
        }
    };

    items
        .into_iter()
        .map(|item| {
            let published_ts = item.published_at.as_deref().and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.timestamp())
            });
            let summary = item.description.unwrap_or_default();
            let image_url = item.cover_image.or(item.social_image);
            let category = item
                .tag_list
                .filter(|tags| !tags.is_empty())
                .map(|tags| format!("#{}", tags.join(" #")))
                .unwrap_or_else(|| "Dev & Tech".to_string());

            ArticleCard {
                id: item.url.clone(),
                title: item.title,
                summary,
                link: item.url,
                published: item.readable_publish_date,
                published_at: published_ts,
                source: "Dev.to".to_string(),
                category,
                image_url,
                reading_time: item.reading_time_minutes,
            }
        })
        .collect()
}

#[derive(Deserialize)]
struct HnItem {
    #[serde(rename = "objectID")]
    object_id: String,
    title: Option<String>,
    url: Option<String>,
    author: Option<String>,
    points: Option<u32>,
    num_comments: Option<u32>,
    created_at_i: Option<i64>,
}

#[derive(Deserialize)]
struct HnResponse {
    hits: Vec<HnItem>,
}

async fn fetch_hackernews(client: &reqwest::Client) -> Vec<ArticleCard> {
    let url = "https://hn.algolia.com/api/v1/search?tags=front_page&hitsPerPage=25";
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to fetch Hacker News: {e}");
            return Vec::new();
        }
    };

    let hn_resp: HnResponse = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to parse Hacker News json: {e}");
            return Vec::new();
        }
    };

    hn_resp
        .hits
        .into_iter()
        .filter_map(|item| {
            let title = item.title?.trim().to_string();
            if title.is_empty() {
                return None;
            }

            let link = item.url.unwrap_or_else(|| {
                format!("https://news.ycombinator.com/item?id={}", item.object_id)
            });

            // Filter out direct PDF links
            if link.to_lowercase().ends_with(".pdf") {
                return None;
            }

            let points = item.points.unwrap_or(0);
            let comments = item.num_comments.unwrap_or(0);
            let author = item.author.unwrap_or_else(|| "community".to_string());
            let summary = format!("{points} points by {author} • {comments} comments");

            let published = item.created_at_i.and_then(|ts| {
                chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.format("%d %b %Y").to_string())
            });

            Some(ArticleCard {
                id: link.clone(),
                title,
                summary,
                link,
                published,
                published_at: item.created_at_i,
                source: "Hacker News".to_string(),
                category: "Tech & Essays".to_string(),
                image_url: None,
                reading_time: None,
            })
        })
        .collect()
}

// --- RSS Feed Fetcher ---------------------------------------------------------

async fn fetch_rss_sources(client: &reqwest::Client, sources: &[&SourceConfig]) -> Vec<ArticleCard> {
    let mut cards = Vec::new();

    for src in sources {
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

                            // Detect image url
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

                            // Published date and unix timestamp
                            let dt_opt = entry.published.or(entry.updated);
                            let published = dt_opt.map(|dt| dt.format("%d %b %Y").to_string());
                            let published_at = dt_opt.map(|dt| dt.timestamp());

                            cards.push(ArticleCard {
                                id: link.clone(),
                                title,
                                summary,
                                link,
                                published,
                                published_at,
                                source: src.name.to_string(),
                                category: src.category.to_string(),
                                image_url,
                                reading_time: None,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(source = src.name, error = %e, "Failed to fetch RSS feed");
            }
        }
    }

    cards
}

// --- Main Feed Provider -------------------------------------------------------

/// Fetch articles feed from curated sources or a specific source
pub async fn get_feed(source_filter: Option<String>) -> Result<Vec<ArticleCard>, String> {
    let now = Instant::now();
    let filter = source_filter.unwrap_or_else(|| "all".to_string()).to_lowercase();

    // Check cache first
    {
        let cache = CACHE.read().await;
        if let Some((fetched_at, items)) = cache.get(&filter) {
            if now.duration_since(*fetched_at) < CACHE_TTL {
                return Ok(items.clone());
            }
        }
    }

    let client = build_http_client()?;
    let mut all_cards = Vec::new();

    match filter.as_str() {
        "devto" => {
            all_cards = fetch_devto(&client).await;
        }
        "hackernews" => {
            all_cards = fetch_hackernews(&client).await;
        }
        "freecodecamp" => {
            let matching: Vec<&SourceConfig> = RSS_SOURCES.iter().filter(|s| s.key == "freecodecamp").collect();
            all_cards = fetch_rss_sources(&client, &matching).await;
        }
        "npr" => {
            let matching: Vec<&SourceConfig> = RSS_SOURCES.iter().filter(|s| s.key == "npr").collect();
            all_cards = fetch_rss_sources(&client, &matching).await;
        }
        "guardian" => {
            let matching: Vec<&SourceConfig> = RSS_SOURCES.iter().filter(|s| s.key == "guardian").collect();
            all_cards = fetch_rss_sources(&client, &matching).await;
        }
        "bbc" => {
            let matching: Vec<&SourceConfig> = RSS_SOURCES.iter().filter(|s| s.key == "bbc").collect();
            all_cards = fetch_rss_sources(&client, &matching).await;
        }
        _ => {
            // "all" or general: fetch all in parallel
            let all_rss: Vec<&SourceConfig> = RSS_SOURCES.iter().collect();
            let (devto_res, hn_res, rss_res) = tokio::join!(
                fetch_devto(&client),
                fetch_hackernews(&client),
                fetch_rss_sources(&client, &all_rss)
            );

            // Interleave/combine all sources
            all_cards.extend(devto_res);
            all_cards.extend(hn_res);
            all_cards.extend(rss_res);

            // Sort chronologically (newest published timestamp first)
            all_cards.sort_by(|a, b| {
                b.published_at.unwrap_or(0).cmp(&a.published_at.unwrap_or(0))
            });
        }
    }

    // Cache the result
    {
        let mut cache = CACHE.write().await;
        cache.insert(filter, (Instant::now(), all_cards.clone()));
    }

    Ok(all_cards)
}

// --- 3-Step Local Pipeline for Article Extraction -----------------------------

/// Detect if response is a paywall, DataDome, Cloudflare, or AWS WAF bot-challenge stub
fn detect_paywall_or_bot_block(html: &str, text: &str) -> bool {
    let lower_html = html.to_lowercase();
    let lower_text = text.to_lowercase();

    lower_html.contains("geo.captcha-delivery.com")
        || lower_html.contains("cf-browser-verification")
        || lower_html.contains("awswaf")
        || lower_html.contains("challenge-running")
        || lower_text.contains("please enable js")
        || lower_text.contains("please enable javascript")
        || lower_text.contains("disable any ad blocker")
        || lower_text.contains("human verification")
        || lower_text.contains("verify you are human")
        || (text.trim().len() < 160 && (lower_html.contains("paywall") || lower_html.contains("subscribe to continue") || lower_html.contains("subscription required")))
}

/// Extract clean article content from any given URL using the 3-step pipeline:
/// 1. reqwest with browser headers
/// 2. Mozilla Readability in native Rust
/// 3. htmd HTML to Markdown conversion
pub async fn extract(url_str: &str) -> Result<ArticleContent, String> {
    let parsed_url = Url::parse(url_str).map_err(|e| format!("Invalid URL: {e}"))?;
    let client = build_http_client()?;

    let response = client
        .get(parsed_url.as_str())
        .send()
        .await
        .map_err(|e| format!("Could not download article: {e}"))?;

    let html_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    let raw_html = String::from_utf8_lossy(&html_bytes);

    // Step 2: Extract with Mozilla Readability in Rust
    let mut cursor = std::io::Cursor::new(&html_bytes);
    let product = readability::extractor::extract(&mut cursor, &parsed_url)
        .map_err(|e| format!("Failed to parse article readability: {e}"))?;

    // Check for anti-bot or paywall blocks
    let is_paywall_blocked = detect_paywall_or_bot_block(&raw_html, &product.text);

    // Step 3: Convert clean HTML to clean Markdown with htmd
    let markdown = htmd::convert(&product.content).unwrap_or_else(|_| product.text.clone());

    // Determine site / source name from URL host
    let source_name = parsed_url
        .host_str()
        .map(|h| {
            let h = h.trim_start_matches("www.").trim_start_matches("feeds.");
            if h.contains("dev.to") {
                "Dev.to"
            } else if h.contains("ycombinator") {
                "Hacker News"
            } else if h.contains("freecodecamp") {
                "FreeCodeCamp"
            } else if h.contains("npr") {
                "NPR News"
            } else if h.contains("bbc") {
                "BBC"
            } else if h.contains("theguardian") {
                "The Guardian"
            } else {
                h
            }
        })
        .unwrap_or("Web Article")
        .to_string();

    // Look for top hero image in raw HTML or content
    let hero_image = extract_og_image(&raw_html).or_else(|| extract_first_img(&product.content));
    let sanitized_html = sanitize_article_html(&product.content, &parsed_url);
    let reading_time_minutes = (product.text.len() / 900).max(1);

    Ok(ArticleContent {
        url: url_str.to_string(),
        title: product.title,
        byline: None,
        source: source_name,
        published: None,
        hero_image,
        text_length: product.text.len(),
        reading_time_minutes,
        is_paywall_blocked,
        html: sanitized_html,
        markdown,
    })
}

/// Helper to extract og:image from raw HTML
fn extract_og_image(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<meta\s+property=["']og:image["']\s+content=["']([^"']+)["']"#).ok()?;
    re.captures(html).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

/// Helper to extract the first img src from HTML content
fn extract_first_img(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"<img[^>]+src=["']([^"']+)["']"#).ok()?;
    re.captures(html).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

/// Sanitize and enhance article HTML:
/// - Resolves relative image and link URLs to absolute URLs
fn sanitize_article_html(html: &str, base_url: &Url) -> String {
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

    #[test]
    fn test_htmd_pipeline() {
        let html = "<h1>Title</h1><p>This is a paragraph with <b>bold text</b>.</p>";
        let md = htmd::convert(html).unwrap();
        assert!(md.contains("# Title"));
        assert!(md.contains("**bold text**"));
    }

    #[test]
    fn test_paywall_detection() {
        let stub = "<p id=\"cmsg\">Please enable JS and disable any ad blocker</p>";
        assert!(detect_paywall_or_bot_block(stub, "Please enable JS and disable any ad blocker"));
    }

    #[tokio::test]
    async fn test_devto_and_hn_fetching() {
        let client = build_http_client().unwrap();
        let devto = fetch_devto(&client).await;
        assert!(!devto.is_empty(), "Dev.to items should not be empty");
        assert_eq!(devto[0].source, "Dev.to");

        let hn = fetch_hackernews(&client).await;
        assert!(!hn.is_empty(), "Hacker News items should not be empty");
        assert_eq!(hn[0].source, "Hacker News");
    }
}
