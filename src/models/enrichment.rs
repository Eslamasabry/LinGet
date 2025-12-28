use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::{Package, PackageEnrichment, PackageSource};

#[derive(Clone, Serialize, Deserialize)]
struct CacheEntry {
    enrichment: PackageEnrichment,
    fetched_at: u64,
}

static ENRICHMENT_CACHE: Lazy<RwLock<Option<HashMap<String, CacheEntry>>>> =
    Lazy::new(|| RwLock::new(None));

/// Cache TTL: 7 days
const CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// HTTP client with timeout
fn http_client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("LinGet/0.1 (Linux Package Manager)")
        .build()?)
}

fn cache_path() -> PathBuf {
    super::Config::config_dir().join("enrichment_cache.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn load_cache() {
    let path = cache_path();
    if !path.exists() {
        return;
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            if let Ok(cache) = serde_json::from_str::<HashMap<String, CacheEntry>>(&content) {
                let now = now_secs();
                let valid: HashMap<_, _> = cache
                    .into_iter()
                    .filter(|(_, v)| now - v.fetched_at < CACHE_TTL_SECS)
                    .collect();

                *ENRICHMENT_CACHE.write() = Some(valid);
                tracing::debug!("Loaded enrichment cache");
            }
        }
        Err(e) => tracing::debug!("Failed to load enrichment cache: {}", e),
    }
}

fn save_cache_to_disk() {
    let cache = ENRICHMENT_CACHE.read().clone();

    if let Some(ref map) = cache {
        let path = cache_path();
        if let Ok(content) = serde_json::to_string(map) {
            let _ = std::fs::write(path, content);
        }
    }
}

fn get_cached(package: &Package) -> Option<PackageEnrichment> {
    let key = format!("{}:{}", package.source, package.name);
    let now = now_secs();

    let cache = ENRICHMENT_CACHE.read();
    if let Some(ref map) = *cache {
        if let Some(entry) = map.get(&key) {
            if now - entry.fetched_at < CACHE_TTL_SECS {
                return Some(entry.enrichment.clone());
            }
        }
    }
    None
}

fn cache_enrichment(package: &Package, enrichment: &PackageEnrichment) {
    let key = format!("{}:{}", package.source, package.name);

    {
        let mut cache = ENRICHMENT_CACHE.write();
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(
            key,
            CacheEntry {
                enrichment: enrichment.clone(),
                fetched_at: now_secs(),
            },
        );
    }

    std::thread::spawn(save_cache_to_disk);
}

pub async fn fetch_enrichment(package: &Package) -> Option<PackageEnrichment> {
    if let Some(cached) = get_cached(package) {
        return Some(cached);
    }

    let enrichment = match package.source {
        PackageSource::Flatpak => fetch_flathub(&package.name).await,
        PackageSource::Cargo => fetch_crates_io(&package.name).await,
        PackageSource::Pip | PackageSource::Pipx => fetch_pypi(&package.name).await,
        PackageSource::Npm => fetch_npmjs(&package.name).await,
        PackageSource::Snap => fetch_snapcraft(&package.name).await,
        PackageSource::Dart => fetch_pub_dev(&package.name).await,
        _ => None,
    };

    if let Some(ref e) = enrichment {
        cache_enrichment(package, e);
    }

    enrichment
}

// ============================================================================
// Flatpak API (for Flatpak packages)
// ============================================================================

#[derive(Deserialize)]
#[allow(dead_code)]
struct FlatpakApp {
    #[serde(rename = "appId")]
    app_id: Option<String>,
    name: Option<String>,
    summary: Option<String>,
    #[serde(rename = "developerName")]
    developer_name: Option<String>,
    icon: Option<String>,
    #[serde(rename = "installs_last_month")]
    installs: Option<u64>,
    categories: Option<Vec<FlatpakCategory>>,
    screenshots: Option<Vec<FlatpakScreenshot>>,
    urls: Option<FlatpakUrls>,
}

#[derive(Deserialize)]
struct FlatpakCategory {
    name: Option<String>,
}

#[derive(Deserialize)]
struct FlatpakScreenshot {
    #[serde(rename = "imgDesktopUrl")]
    img_desktop_url: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FlatpakUrls {
    homepage: Option<String>,
    bugtracker: Option<String>,
}

async fn fetch_flathub(app_id: &str) -> Option<PackageEnrichment> {
    let client = http_client()
        .context("Failed to create HTTP client for Flatpak")
        .ok()?;
    let url = format!("https://flathub.org/api/v2/appstream/{}", app_id);

    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch Flatpak metadata")
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let app: FlatpakApp = resp
        .json()
        .await
        .context("Failed to parse Flatpak app data")
        .ok()?;

    let icon_url = app.icon.map(|i| {
        if i.starts_with("http") {
            i
        } else {
            format!(
                "https://dl.flathub.org/repo/appstream/x86_64/icons/128x128/{}.png",
                app_id
            )
        }
    });

    let screenshots: Vec<String> = app
        .screenshots
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| s.img_desktop_url)
        .take(5)
        .collect();

    let categories: Vec<String> = app
        .categories
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.name)
        .collect();

    Some(PackageEnrichment {
        icon_url,
        screenshots,
        categories,
        developer: app.developer_name,
        downloads: app.installs,
        summary: app.summary,
        repository: app.urls.and_then(|u| u.homepage),
        ..Default::default()
    })
}

// ============================================================================
// crates.io API (for Cargo packages)
// ============================================================================

#[derive(Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    krate: CratesIoCrate,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CratesIoCrate {
    name: Option<String>,
    description: Option<String>,
    downloads: Option<u64>,
    repository: Option<String>,
    homepage: Option<String>,
    documentation: Option<String>,
    categories: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    updated_at: Option<String>,
}

async fn fetch_crates_io(name: &str) -> Option<PackageEnrichment> {
    let client = http_client()
        .context("Failed to create HTTP client for crates.io")
        .ok()?;
    let url = format!("https://crates.io/api/v1/crates/{}", name);

    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch crates.io metadata")
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: CratesIoResponse = resp
        .json()
        .await
        .context("Failed to parse crates.io response")
        .ok()?;
    let c = data.krate;

    Some(PackageEnrichment {
        summary: c.description,
        downloads: c.downloads,
        repository: c.repository.or(c.homepage),
        keywords: c.keywords.unwrap_or_default(),
        categories: c.categories.unwrap_or_default(),
        last_updated: c.updated_at,
        ..Default::default()
    })
}

// ============================================================================
// PyPI API (for pip/pipx packages)
// ============================================================================

#[derive(Deserialize)]
struct PyPIResponse {
    info: PyPIInfo,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct PyPIInfo {
    summary: Option<String>,
    author: Option<String>,
    author_email: Option<String>,
    home_page: Option<String>,
    project_url: Option<String>,
    project_urls: Option<HashMap<String, String>>,
    keywords: Option<String>,
    classifiers: Option<Vec<String>>,
}

async fn fetch_pypi(name: &str) -> Option<PackageEnrichment> {
    let client = http_client()
        .context("Failed to create HTTP client for PyPI")
        .ok()?;
    let url = format!("https://pypi.org/pypi/{}/json", name);

    let resp = client.get(&url).send().await;
    if let Err(ref e) = resp {
        tracing::debug!("Failed to fetch PyPI metadata: {}", e);
        return None;
    }
    let resp = resp.unwrap();
    if !resp.status().is_success() {
        return None;
    }

    let data: PyPIResponse = resp
        .json()
        .await
        .context("Failed to parse PyPI response")
        .ok()?;
    let info = data.info;

    // Extract repository from project_urls
    let repository = info
        .project_urls
        .as_ref()
        .and_then(|urls| {
            urls.get("Repository")
                .or_else(|| urls.get("Source"))
                .or_else(|| urls.get("GitHub"))
                .cloned()
        })
        .or(info.home_page);

    // Parse keywords
    let keywords: Vec<String> = info
        .keywords
        .map(|k| k.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    // Extract categories from classifiers
    let categories: Vec<String> = info
        .classifiers
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.starts_with("Topic ::"))
        .map(|c| c.split(" :: ").nth(1).unwrap_or(&c).to_string())
        .take(5)
        .collect();

    Some(PackageEnrichment {
        summary: info.summary,
        developer: info.author,
        repository,
        keywords,
        categories,
        ..Default::default()
    })
}

// ============================================================================
// npmjs API (for npm packages)
// ============================================================================

#[derive(Deserialize)]
struct NpmResponse {
    description: Option<String>,
    keywords: Option<Vec<String>>,
    repository: Option<NpmRepository>,
    homepage: Option<String>,
    author: Option<NpmAuthor>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NpmRepository {
    String(String),
    Object { url: Option<String> },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NpmAuthor {
    String(String),
    Object { name: Option<String> },
}

async fn fetch_npmjs(name: &str) -> Option<PackageEnrichment> {
    let client = http_client()
        .context("Failed to create HTTP client for npmjs")
        .ok()?;
    let url = format!("https://registry.npmjs.org/{}", name);

    let resp = client.get(&url).send().await;
    if let Err(ref e) = resp {
        tracing::debug!("Failed to fetch npmjs registry metadata: {}", e);
        return None;
    }
    let resp = resp.unwrap();
    if !resp.status().is_success() {
        return None;
    }

    let data: NpmResponse = resp
        .json()
        .await
        .context("Failed to parse npmjs registry response")
        .ok()?;

    let developer = match data.author {
        Some(NpmAuthor::String(s)) => Some(s),
        Some(NpmAuthor::Object { name }) => name,
        None => None,
    };

    let repository = match data.repository {
        Some(NpmRepository::String(s)) => Some(s),
        Some(NpmRepository::Object { url }) => url,
        None => data.homepage.clone(),
    };

    Some(PackageEnrichment {
        summary: data.description,
        keywords: data.keywords.unwrap_or_default(),
        repository,
        developer,
        ..Default::default()
    })
}

// ============================================================================
// Snapcraft API (for Snap packages)
// ============================================================================

#[derive(Deserialize)]
struct SnapResponse {
    snap: SnapInfo,
}

#[derive(Deserialize)]
struct SnapInfo {
    summary: Option<String>,
    description: Option<String>,
    publisher: Option<SnapPublisher>,
    media: Option<Vec<SnapMedia>>,
    categories: Option<Vec<SnapCategory>>,
    website: Option<String>,
}

#[derive(Deserialize)]
struct SnapPublisher {
    #[serde(rename = "display-name")]
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct SnapMedia {
    #[serde(rename = "type")]
    media_type: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
struct SnapCategory {
    name: Option<String>,
}

async fn fetch_snapcraft(name: &str) -> Option<PackageEnrichment> {
    let client = http_client()
        .context("Failed to create HTTP client for Snapcraft")
        .ok()?;
    let url = format!("https://api.snapcraft.io/v2/snaps/info/{}", name);

    let resp = client
        .get(&url)
        .header("Snap-Device-Series", "16")
        .send()
        .await;
    if let Err(ref e) = resp {
        tracing::debug!("Failed to fetch snap metadata: {}", e);
        return None;
    }
    let resp = resp.unwrap();
    if !resp.status().is_success() {
        return None;
    }

    let data: SnapResponse = resp
        .json()
        .await
        .context("Failed to parse snap response")
        .ok()?;
    let snap = data.snap;

    let icon_url = snap.media.as_ref().and_then(|m| {
        m.iter()
            .find(|i| i.media_type.as_deref() == Some("icon"))
            .and_then(|i| i.url.clone())
    });

    let screenshots: Vec<String> = snap
        .media
        .unwrap_or_default()
        .into_iter()
        .filter(|m| m.media_type.as_deref() == Some("screenshot"))
        .filter_map(|m| m.url)
        .take(5)
        .collect();

    let categories: Vec<String> = snap
        .categories
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.name)
        .collect();

    let developer = snap.publisher.and_then(|p| p.display_name);

    Some(PackageEnrichment {
        icon_url,
        screenshots,
        categories,
        summary: snap.summary.or(snap.description),
        developer,
        repository: snap.website,
        ..Default::default()
    })
}

// ============================================================================
// pub.dev API (for Dart packages)
// ============================================================================

#[derive(Deserialize)]
#[allow(dead_code)]
struct PubDevResponse {
    name: Option<String>,
    latest: Option<PubDevVersion>,
}

#[derive(Deserialize)]
struct PubDevVersion {
    pubspec: Option<PubDevPubspec>,
}

#[derive(Deserialize)]
struct PubDevPubspec {
    description: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
}

#[derive(Deserialize)]
struct PubDevScore {
    #[serde(rename = "likeCount")]
    like_count: Option<u64>,
    #[serde(rename = "popularityScore")]
    popularity_score: Option<f32>,
}

async fn fetch_pub_dev(name: &str) -> Option<PackageEnrichment> {
    let client = http_client()
        .context("Failed to create HTTP client for pub.dev")
        .ok()?;

    // Fetch package info
    let url = format!("https://pub.dev/api/packages/{}", name);
    let resp = client.get(&url).send().await;
    if let Err(ref e) = resp {
        tracing::debug!("Failed to fetch pub.dev package info: {}", e);
        return None;
    }
    let resp = resp.unwrap();
    if !resp.status().is_success() {
        return None;
    }

    let data: PubDevResponse = resp
        .json()
        .await
        .context("Failed to fetch pub.dev package metadata")
        .ok()?;
    let pubspec = data.latest.and_then(|v| v.pubspec)?;

    // Fetch score
    let score_url = format!("https://pub.dev/api/packages/{}/score", name);
    let score: Option<PubDevScore> = match client.get(&score_url).send().await {
        Ok(r) => r
            .json()
            .await
            .context("Failed to parse pub.dev score response")
            .ok(),
        Err(_) => None,
    };

    let rating = score
        .as_ref()
        .and_then(|s| s.popularity_score)
        .map(|p| p * 5.0); // Convert 0-1 to 0-5

    let downloads = score.and_then(|s| s.like_count);

    Some(PackageEnrichment {
        summary: pubspec.description,
        repository: pubspec.repository.or(pubspec.homepage),
        rating,
        downloads,
        ..Default::default()
    })
}

/// Batch enrich multiple packages (runs in parallel with rate limiting)
#[allow(dead_code)]
pub async fn enrich_packages(packages: &mut [Package], max_concurrent: usize) {
    use futures::stream::{self, StreamExt};

    let enrichments: Vec<_> = stream::iter(packages.iter())
        .map(|p| async move {
            let enrichment = fetch_enrichment(p).await;
            (p.id(), enrichment)
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await;

    // Apply enrichments
    let enrichment_map: HashMap<_, _> = enrichments.into_iter().collect();
    for pkg in packages.iter_mut() {
        if let Some(Some(enrichment)) = enrichment_map.get(&pkg.id()) {
            pkg.enrichment = Some(enrichment.clone());
        }
    }
}
