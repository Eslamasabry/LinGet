//! The LinGet web dashboard: `linget web` serves a three.js package
//! galaxy on the local network, designed for phone access over a tailnet.
//!
//! The server embeds every asset (vendored three.js included) so the binary
//! stays self-contained. Catalog data is served cache-first — the same
//! radar catalog cache the TUI writes — so a warm start paints instantly
//! while a fresh check runs in the background.

use crate::backend::history_tracker::HistoryTracker;
use crate::backend::streaming::StreamLine;
use crate::backend::{PackageManager, TaskQueueEvent, TaskQueueExecutor};
use crate::cli::tui_next::cache as catalog_cache;
use crate::models::history::{TaskQueueAction, TaskQueueEntry, TaskQueueStatus};
use crate::models::{Config, Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        Html, IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::StreamExt;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

mod assets;

/// Live event pushed to connected browsers over SSE.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WebQueueEvent {
    /// A task started, completed or failed (full entry state).
    Task(Box<TaskQueueEntry>),
    /// One streamed output line from a running task.
    Log { entry_id: String, line: String },
}

pub struct WebState {
    pm: Arc<RwLock<PackageManager>>,
    history: Arc<Mutex<Option<HistoryTracker>>>,
    packages: RwLock<Vec<Package>>,
    generated_at: RwLock<Option<chrono::DateTime<chrono::Local>>>,
    favorites: RwLock<HashSet<String>>,
    refreshing: AtomicBool,
    executor_running: Arc<AtomicBool>,
    /// Fan-out for queue events to all connected browsers.
    events: broadcast::Sender<WebQueueEvent>,
}

/// Sources whose queue tasks run as verified transactions with an attached
/// reviewed plan — same contract as the TUI queue.
fn stable_transaction_source(source: PackageSource) -> bool {
    matches!(
        source,
        PackageSource::Apt | PackageSource::Flatpak | PackageSource::Npm
    )
}

pub async fn run(bind_addr: &str, port: u16) -> Result<()> {
    let addr: SocketAddr = format!("{bind_addr}:{port}")
        .parse()
        .with_context(|| format!("invalid bind address {bind_addr}:{port}"))?;

    let pm = Arc::new(RwLock::new(PackageManager::new_fast()));
    let tracker = HistoryTracker::load().await.ok();
    let history = Arc::new(Mutex::new(tracker));
    let (events, _) = broadcast::channel(512);

    let state = Arc::new(WebState {
        pm: pm.clone(),
        history: history.clone(),
        packages: RwLock::new(Vec::new()),
        generated_at: RwLock::new(None),
        favorites: RwLock::new(
            Config::load()
                .favorite_packages
                .into_iter()
                .collect::<HashSet<_>>(),
        ),
        refreshing: AtomicBool::new(false),
        executor_running: Arc::new(AtomicBool::new(false)),
        events,
    });

    // Cache-first: paint the last catalog immediately, then revalidate.
    if let Some(cached) = catalog_cache::load() {
        *state.packages.write().await = cached.packages;
        *state.generated_at.write().await = Some(cached.saved_at);
    }
    spawn_refresh(state.clone());

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/app.js", get(app_js))
        .route("/style.css", get(style_css))
        .route("/vendor/{*path}", get(vendor))
        .route("/api/catalog", get(catalog))
        .route("/api/refresh", post(refresh))
        .route("/api/queue", get(queue_state).post(enqueue))
        .route("/api/queue/stream", get(queue_stream))
        .route("/api/queue/retry", post(retry_failed))
        .route("/api/favorites", post(toggle_favorite))
        .route("/api/changelog", get(changelog))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;
    println!("linget web listening on http://{addr}");
    println!(
        "open from your phone over tailscale at http://{}:{port}",
        primary_local_ip()
    );
    if !addr.ip().is_loopback() {
        println!("note: bound on a non-loopback interface — anyone on that network can drive package operations");
    }

    axum::serve(listener, app)
        .await
        .context("web server failed")
}

/// Best-effort default-route source address, which on a tailscale host is
/// usually the tailnet IP — the one address a phone can actually reach.
fn primary_local_ip() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("100.100.100.100:1")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

// ----------------------------------------------------------------------
// Static assets
// ----------------------------------------------------------------------

async fn index() -> Html<&'static str> {
    Html(assets::INDEX_HTML)
}

async fn app_js() -> Response {
    javascript(assets::APP_JS)
}

async fn style_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        assets::STYLE_CSS,
    )
        .into_response()
}

fn javascript(body: &'static str) -> Response {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        body,
    )
        .into_response()
}

async fn vendor(Path(path): Path<String>) -> Response {
    let file = match path.as_str() {
        "three.module.js" => assets::THREE_JS,
        "addons/controls/OrbitControls.js" => assets::ORBIT_CONTROLS,
        "addons/postprocessing/EffectComposer.js" => assets::EFFECT_COMPOSER,
        "addons/postprocessing/RenderPass.js" => assets::RENDER_PASS,
        "addons/postprocessing/ShaderPass.js" => assets::SHADER_PASS,
        "addons/postprocessing/MaskPass.js" => assets::MASK_PASS,
        "addons/postprocessing/Pass.js" => assets::PASS,
        "addons/postprocessing/UnrealBloomPass.js" => assets::UNREAL_BLOOM,
        "addons/postprocessing/OutputPass.js" => assets::OUTPUT_PASS,
        "addons/shaders/CopyShader.js" => assets::COPY_SHADER,
        "addons/shaders/LuminosityHighPassShader.js" => assets::LUMINOSITY_HIGH_PASS,
        "addons/shaders/OutputShader.js" => assets::OUTPUT_SHADER,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    javascript(file)
}

async fn health() -> &'static str {
    "ok"
}

// ----------------------------------------------------------------------
// Catalog
// ----------------------------------------------------------------------

#[derive(Serialize)]
struct CatalogPackage {
    /// Stable `source:name` identity — Package::id() is a method, not a
    /// serialized field, so the API must send it explicitly.
    id: String,
    #[serde(flatten)]
    package: Package,
    is_favorite: bool,
    is_security: bool,
}

#[derive(Serialize)]
struct CatalogCounts {
    updates: usize,
    security: usize,
    installed: usize,
}

#[derive(Serialize)]
struct ProviderCount {
    source: String,
    label: String,
    count: usize,
}

#[derive(Serialize)]
struct CatalogResponse {
    refreshing: bool,
    generated_at: Option<chrono::DateTime<chrono::Local>>,
    counts: CatalogCounts,
    providers: Vec<ProviderCount>,
    packages: Vec<CatalogPackage>,
}

async fn catalog(State(state): State<Arc<WebState>>) -> Json<CatalogResponse> {
    let favorites = state.favorites.read().await.clone();
    let packages = state.packages.read().await;
    let mut updates = 0;
    let mut security = 0;
    let mut installed = 0;
    let mut by_source: HashMap<PackageSource, usize> = HashMap::new();
    for package in packages.iter() {
        by_source.entry(package.source).or_default();
        *by_source.get_mut(&package.source).unwrap() += 1;
        match package.status {
            PackageStatus::UpdateAvailable => {
                updates += 1;
                if package.detect_update_category() == crate::models::UpdateCategory::Security {
                    security += 1;
                }
            }
            PackageStatus::Installed => installed += 1,
            _ => {}
        }
    }
    let counts = CatalogCounts {
        updates,
        security,
        installed,
    };
    let mut providers: Vec<ProviderCount> = by_source
        .into_iter()
        .map(|(source, count)| ProviderCount {
            label: source.to_string().to_lowercase(),
            source: source.to_string(),
            count,
        })
        .collect();
    providers.sort_by(|a, b| b.count.cmp(&a.count).then(a.label.cmp(&b.label)));

    Json(CatalogResponse {
        refreshing: state.refreshing.load(Ordering::Relaxed),
        generated_at: *state.generated_at.read().await,
        counts,
        providers,
        packages: packages
            .iter()
            .map(|package| CatalogPackage {
                id: package.id(),
                package: package.clone(),
                is_favorite: favorites.contains(&package.id()),
                is_security: package.status == PackageStatus::UpdateAvailable
                    && package.detect_update_category() == crate::models::UpdateCategory::Security,
            })
            .collect(),
    })
}

async fn refresh(State(state): State<Arc<WebState>>) -> StatusCode {
    spawn_refresh(state);
    StatusCode::ACCEPTED
}

fn spawn_refresh(state: Arc<WebState>) {
    if state
        .refreshing
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return; // already refreshing
    }
    tokio::spawn(async move {
        let loaded = load_catalog_fresh(state.pm.clone()).await;
        match loaded {
            Ok(packages) => {
                *state.packages.write().await = packages.clone();
                *state.generated_at.write().await = Some(chrono::Local::now());
                catalog_cache::save_async(packages);
            }
            Err(error) => {
                tracing::warn!(error = %error, "web catalog refresh failed");
            }
        }
        state.refreshing.store(false, Ordering::Relaxed);
    });
}

async fn load_catalog_fresh(pm: Arc<RwLock<PackageManager>>) -> Result<Vec<Package>> {
    let guard = pm.read().await;
    let mut packages = guard.list_all_installed().await?;
    let index_by_id: HashMap<String, usize> = packages
        .iter()
        .enumerate()
        .map(|(index, package)| (package.id(), index))
        .collect();
    if let Ok(updates) = guard.check_all_updates().await {
        for update in updates {
            if let Some(&index) = index_by_id.get(&update.id()) {
                let existing = &mut packages[index];
                existing.status = PackageStatus::UpdateAvailable;
                existing.available_version = update
                    .available_version
                    .or_else(|| Some(update.version.clone()));
                existing.update_category = update.update_category;
            }
        }
    }
    Ok(packages)
}

// ----------------------------------------------------------------------
// Queue
// ----------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct EnqueueRequest {
    action: String,
    ids: Vec<String>,
}

#[derive(Serialize)]
struct EnqueueResponse {
    queued: usize,
    planned: usize,
}

async fn enqueue(
    State(state): State<Arc<WebState>>,
    Json(request): Json<EnqueueRequest>,
) -> Result<Json<EnqueueResponse>, Response> {
    let action = match request.action.as_str() {
        "update" => TaskQueueAction::Update,
        "remove" => TaskQueueAction::Remove,
        "install" => TaskQueueAction::Install,
        other => {
            return Err(
                (StatusCode::BAD_REQUEST, format!("unknown action {other:?}")).into_response(),
            )
        }
    };

    let packages = state.packages.read().await.clone();
    let selected: Vec<Package> = request
        .ids
        .iter()
        .filter_map(|id| packages.iter().find(|p| &p.id() == id))
        .cloned()
        .collect();

    let (queued, planned) = enqueue_packages(&state, selected, action)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response())?;
    Ok(Json(EnqueueResponse { queued, planned }))
}

/// Enqueues per-package entries for ordinary sources and per-source planned
/// transaction entries for stable sources — the same split the TUI makes.
async fn enqueue_packages(
    state: &WebState,
    selected: Vec<Package>,
    action: TaskQueueAction,
) -> Result<(usize, usize)> {
    let mut stable: HashMap<PackageSource, Vec<crate::backend::transaction::PackageRef>> =
        HashMap::new();
    let mut entries = Vec::new();
    let queue = current_queue(state).await;

    for package in selected {
        let allowed = match action {
            TaskQueueAction::Update => package.status == PackageStatus::UpdateAvailable,
            TaskQueueAction::Remove => matches!(
                package.status,
                PackageStatus::Installed | PackageStatus::UpdateAvailable
            ),
            TaskQueueAction::Install => package.status == PackageStatus::NotInstalled,
        };
        if !allowed {
            continue;
        }
        if stable_transaction_source(package.source) {
            stable.entry(package.source).or_default().push(
                crate::backend::transaction::PackageRef::from_package(&package),
            );
            continue;
        }
        let already = queue.iter().any(|entry| {
            entry.package_id == package.id()
                && matches!(
                    entry.status,
                    TaskQueueStatus::Queued | TaskQueueStatus::Running
                )
        });
        if already {
            continue;
        }
        entries.push(TaskQueueEntry::new(
            action,
            package.id(),
            package.name.clone(),
            package.source,
        ));
    }

    let planned_targets: usize = stable.values().map(|targets| targets.len()).sum();
    let mut planned_entries = Vec::new();
    if !stable.is_empty() {
        planned_entries = plan_stable_transactions(state, action, stable).await?;
    }

    let queued = entries.len();
    {
        let mut guard = state.history.lock().await;
        let tracker = guard.as_mut().context("history tracker not initialized")?;
        for entry in entries.into_iter().chain(planned_entries) {
            tracker.enqueue_task(entry).await?;
        }
        tracker.save().await?;
    }
    ensure_executor(state).await;
    Ok((queued, planned_targets))
}

async fn plan_stable_transactions(
    state: &WebState,
    action: TaskQueueAction,
    groups: HashMap<PackageSource, Vec<crate::backend::transaction::PackageRef>>,
) -> Result<Vec<TaskQueueEntry>> {
    use crate::backend::transaction::{
        OperationAction, OperationRequest, RequestedBy, RiskLevel, TransactionEngine,
    };

    let store = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("linget")
        .join("transactions.json");
    let engine = TransactionEngine::load(state.pm.clone(), store)
        .await
        .map_err(|error| anyhow::anyhow!(error.safe_message))?;
    let operation_action = match action {
        TaskQueueAction::Install => OperationAction::Install,
        TaskQueueAction::Remove => OperationAction::Remove,
        TaskQueueAction::Update => OperationAction::Update,
    };

    let mut sources: Vec<_> = groups.into_iter().collect();
    sources.sort_by_key(|(source, _)| source.to_string());
    let mut entries = Vec::new();
    for (_source, targets) in sources {
        let request = OperationRequest::new(operation_action, targets, RequestedBy::Tui);
        let (plan, risk) = engine
            .plan(request)
            .await
            .map_err(|error| anyhow::anyhow!(error.safe_message))?;
        if risk.level == RiskLevel::Blocked {
            anyhow::bail!("{} provider plan is blocked", plan.provider.source);
        }
        let plan_json =
            serde_json::to_string(&plan).context("provider plan could not be serialized")?;
        let package_name = if plan.targets.len() == 1 {
            plan.targets[0].name.clone()
        } else {
            format!("{} {} packages", plan.targets.len(), plan.provider.source)
        };
        let mut entry = TaskQueueEntry::new(
            action,
            format!("transaction:{}", plan.operation_id),
            package_name,
            plan.provider.source,
        );
        entry.reviewed_operation_id = Some(plan.operation_id.clone());
        entry.reviewed_plan_json = Some(plan_json);
        entries.push(entry);
    }
    Ok(entries)
}

#[derive(Serialize)]
struct QueueResponse {
    entries: Vec<TaskQueueEntry>,
}

async fn queue_state(State(state): State<Arc<WebState>>) -> Json<QueueResponse> {
    Json(QueueResponse {
        entries: current_queue(&state).await,
    })
}

async fn current_queue(state: &WebState) -> Vec<TaskQueueEntry> {
    let guard = state.history.lock().await;
    guard
        .as_ref()
        .map(|tracker| tracker.history().task_queue.entries.clone())
        .unwrap_or_default()
}

#[derive(Serialize)]
struct RetryResponse {
    retried: usize,
}

async fn retry_failed(State(state): State<Arc<WebState>>) -> Result<Json<RetryResponse>, Response> {
    let queue = current_queue(&state).await;
    // Latest failure per (package, action) wins; skip anything active.
    let active: HashSet<(String, u8)> = queue
        .iter()
        .filter(|entry| {
            matches!(
                entry.status,
                TaskQueueStatus::Queued | TaskQueueStatus::Running
            )
        })
        .map(|entry| (entry.package_id.clone(), action_key(entry.action)))
        .collect();
    let mut seen = HashSet::new();
    let mut retries: Vec<TaskQueueEntry> = Vec::new();
    for entry in queue.iter().rev() {
        if entry.status != TaskQueueStatus::Failed {
            continue;
        }
        let key = (entry.package_id.clone(), action_key(entry.action));
        if active.contains(&key) || !seen.insert(key) {
            continue;
        }
        // Retrying cannot fix a policy refusal (PEP 668) — the error text
        // says install via pipx or the distro package instead.
        if let Some(error) = entry.error.as_deref() {
            if crate::models::history::FailureCategory::classify(error)
                == crate::models::history::FailureCategory::ExternallyManaged
            {
                continue;
            }
        }
        // Planless stable failures come from an older build; route them
        // through fresh planning instead of failing the plan guard again.
        if stable_transaction_source(entry.package_source) && entry.reviewed_plan_json.is_none() {
            continue;
        }
        let mut retry = TaskQueueEntry::new(
            entry.action,
            entry.package_id.clone(),
            entry.package_name.clone(),
            entry.package_source,
        );
        retry.reviewed_operation_id = entry.reviewed_operation_id.clone();
        retry.reviewed_plan_json = entry.reviewed_plan_json.clone();
        retry.retry_of = Some(entry.id.clone());
        retries.push(retry);
    }

    let count = retries.len();
    {
        let mut guard = state.history.lock().await;
        let tracker = guard
            .as_mut()
            .context("history tracker not initialized")
            .map_err(|error| {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            })?;
        for entry in retries {
            tracker
                .enqueue_task(entry)
                .await
                .context("failed to re-enqueue task")
                .map_err(|error| {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                })?;
        }
        tracker
            .save()
            .await
            .context("failed to save task queue")
            .map_err(|error| {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            })?;
    }
    ensure_executor(&state).await;
    Ok(Json(RetryResponse { retried: count }))
}

fn action_key(action: TaskQueueAction) -> u8 {
    match action {
        TaskQueueAction::Install => 0,
        TaskQueueAction::Remove => 1,
        TaskQueueAction::Update => 2,
    }
}

async fn ensure_executor(state: &WebState) {
    if state
        .executor_running
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return; // an executor is already draining the queue
    }
    let executor = TaskQueueExecutor::new(state.pm.clone(), state.history.clone());
    let broadcaster = state.events.clone();
    let flag = state.executor_running.clone();

    // The executor emits progress to an mpsc channel; forward everything to
    // the broadcast channel so every connected browser sees live logs.
    let (tx, mut rx) = mpsc::channel::<TaskQueueEvent>(256);
    let forward = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let web_event = match event {
                TaskQueueEvent::Started(entry)
                | TaskQueueEvent::Completed(entry)
                | TaskQueueEvent::Failed(entry) => WebQueueEvent::Task(Box::new(entry)),
                TaskQueueEvent::Log { entry_id, line } => WebQueueEvent::Log {
                    entry_id,
                    line: match line {
                        StreamLine::Stdout(text) | StreamLine::Stderr(text) => text,
                    },
                },
            };
            // A send failure just means nobody is listening; logs are not
            // retained server-side.
            let _ = broadcaster.send(web_event);
        }
    });

    tokio::spawn(async move {
        let _ = executor.run(Some(tx)).await;
        // tx dropped when run() returns, so the forwarder drains and exits.
        let _ = forward.await;
        flag.store(false, Ordering::Relaxed);
    });
}

// ----------------------------------------------------------------------
// Live event stream
// ----------------------------------------------------------------------

async fn queue_stream(State(state): State<Arc<WebState>>) -> impl IntoResponse {
    // Initial snapshot first, so a fresh browser is immediately in sync;
    // anything emitted between snapshot and subscribe is caught by the
    // periodic queue polling already in the client.
    let entries = current_queue(&state).await;
    let initial = Event::default()
        .event("state")
        .json_data(&QueueResponse { entries })
        .expect("queue state serializes");

    let receiver = state.events.subscribe();
    let live = futures::stream::unfold(receiver, |mut rx| async move {
        match rx.recv().await {
            Ok(event) => {
                let sse = Event::default()
                    .event(match event {
                        WebQueueEvent::Task(_) => "task",
                        WebQueueEvent::Log { .. } => "log",
                    })
                    .json_data(&event)
                    .unwrap_or_else(|_| Event::default().comment("serialize failed"));
                Some((Ok::<_, Infallible>(sse), rx))
            }
            // Lagged only means lines were dropped; the client resyncs from
            // the next task/state event.
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok(Event::default().comment("lagged")), rx))
            }
            Err(_) => None,
        }
    });

    Sse::new(futures::stream::once(async move { Ok::<_, Infallible>(initial) }).chain(live))
        .keep_alive(KeepAlive::default())
}

// ----------------------------------------------------------------------
// Favorites & changelog
// ----------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct FavoriteRequest {
    id: String,
}

#[derive(Serialize)]
struct FavoriteResponse {
    is_favorite: bool,
}

async fn toggle_favorite(
    State(state): State<Arc<WebState>>,
    Json(request): Json<FavoriteRequest>,
) -> Json<FavoriteResponse> {
    let mut favorites = state.favorites.write().await;
    let now_favorite = !favorites.contains(&request.id);
    if now_favorite {
        favorites.insert(request.id.clone());
    } else {
        favorites.remove(&request.id);
    }
    let ids: Vec<String> = favorites.iter().cloned().collect();
    drop(favorites);

    tokio::task::spawn_blocking(move || {
        let mut config = Config::load();
        config.favorite_packages = ids;
        if let Err(error) = config.save() {
            tracing::warn!(error = %error, "failed to persist favorites");
        }
    });

    Json(FavoriteResponse {
        is_favorite: now_favorite,
    })
}

#[derive(serde::Deserialize)]
struct ChangelogQuery {
    id: String,
}

async fn changelog(
    State(state): State<Arc<WebState>>,
    Query(query): Query<ChangelogQuery>,
) -> Response {
    let package = {
        let packages = state.packages.read().await;
        packages.iter().find(|p| p.id() == query.id).cloned()
    };
    let Some(package) = package else {
        return (StatusCode::NOT_FOUND, "unknown package").into_response();
    };
    let result = {
        let guard = state.pm.read().await;
        guard.get_changelog(&package).await
    };
    match result {
        Ok(Some(text)) => Json(serde_json::json!({ "changelog": text })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "no changelog").into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("changelog failed: {error}"),
        )
            .into_response(),
    }
}
