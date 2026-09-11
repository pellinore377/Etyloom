pub mod api;
pub mod auth;
pub mod config;
pub mod error;
pub mod jobs;
pub mod store;

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use config::Config;
use etyloom_core::Package;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::{RwLock, Semaphore};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    pub http: reqwest::Client,
    pub cache: Arc<RwLock<BTreeMap<String, Arc<Package>>>>,
    pub interactive: Arc<Semaphore>,
}

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(10));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .context("Opening database")?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Applying database migrations")?;
    Ok(pool)
}

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/api/session", get(auth::session))
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/dev", get(auth::dev_login))
        .route("/auth/logout", post(auth::logout))
        .route(
            "/api/projects",
            get(api::projects).post(api::create_project),
        )
        .route("/api/languages", get(api::languages))
        .route("/api/generate", post(api::generate))
        .route("/api/import", post(api::import))
        .route("/api/jobs", get(api::jobs))
        .route("/api/jobs/{id}", get(api::job))
        .route("/api/jobs/{id}/cancel", post(api::cancel))
        .route("/api/languages/{id}", get(api::language))
        .route("/api/languages/{id}/words", get(api::words))
        .route("/api/languages/{id}/draft", post(api::draft))
        .route("/api/languages/{id}/publish", post(api::publish))
        .route("/api/languages/{id}/translate", post(api::translate))
        .route("/api/languages/{id}/exercise", get(api::exercise))
        .route("/api/languages/{id}/answer", post(api::answer))
        .route("/api/languages/{id}/export", get(api::export))
        .route("/api/languages/{id}/recipe", get(api::recipe))
        .route("/api/languages/{id}/corpus", get(api::corpus))
        .route("/health/live", get(|| async { "ok" }))
        .route("/health/ready", get(ready))
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
}

async fn ready(State(state): State<AppState>) -> error::Result<&'static str> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok("ok")
}

async fn security(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    response
}

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=warn".into()),
        )
        .init();
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        reqwest::Client::new()
            .get("http://127.0.0.1:3000/health/ready")
            .timeout(Duration::from_secs(3))
            .send()
            .await?
            .error_for_status()?;
        return Ok(());
    }
    let config = Config::from_env()?;
    let db = connect(&config.database_url).await?;
    if std::env::args().nth(1).as_deref() == Some("backup") {
        let path = std::env::args()
            .nth(2)
            .context("Provide a new backup filename")?;
        sqlx::query("VACUUM INTO ?").bind(path).execute(&db).await?;
        return Ok(());
    }
    sqlx::query("UPDATE jobs SET state='queued',phase='Resuming saved recipe after restart' WHERE state='running'").execute(&db).await?;
    sqlx::query("UPDATE jobs SET state='canceled',phase='Cancellation completed after restart' WHERE state='cancel_requested'").execute(&db).await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at<=unixepoch()")
        .execute(&db)
        .await?;
    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()?,
        cache: Arc::new(RwLock::new(BTreeMap::new())),
        interactive: Arc::new(Semaphore::new(2)),
    };
    for _ in 0..config.workers {
        tokio::spawn(jobs::worker(state.clone()));
    }
    let options = get_configuration(None)?.leptos_options;
    let routes = generate_route_list(crate::app::App);
    let pages = Router::new()
        .leptos_routes(&options, routes, {
            let options = options.clone();
            move || crate::app::shell(options.clone())
        })
        .with_state(options);
    let app = api_router()
        .with_state(state)
        .merge(pages)
        .fallback_service(ServeDir::new(&config.site_root))
        .layer(middleware::from_fn(security))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(config.address)
        .await
        .context("Binding HTTP listener")?;
    tracing::info!(address = %config.address, development = config.development, "Etyloom is listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                tokio::select! { result = tokio::signal::ctrl_c() => { if let Err(error) = result { tracing::error!(%error, "signal handler failed"); } }, _ = signal.recv() => {} }
            }
            Err(error) => {
                tracing::error!(%error, "SIGTERM handler failed");
                if let Err(error) = tokio::signal::ctrl_c().await {
                    tracing::error!(%error, "signal handler failed");
                }
            }
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "signal handler failed");
    }
}
