mod compilation_worker;
mod handlers;
mod metrics_worker;
mod models;
mod sandbox;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    Router,
    routing::{get, post},
};
use models::Results;
use tokio::sync::Mutex;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::{self, TraceLayer},
};
use tracing::{Level, info};

use crate::models::{AppState, Job};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // Ensure we are not root
    if nix::unistd::Uid::effective().is_root() {
        eprintln!("This server should not be run as root for security reasons.");
        std::process::exit(1);
    }

    info!("Spawning workers...");

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let (tx, rx) = async_channel::unbounded::<Job>();
    let results: Results = Arc::new(Mutex::new(HashMap::new()));

    for i in 0..num_workers {
        let rx = rx.clone();
        let results = results.clone();
        tokio::spawn(async move {
            compilation_worker::worker(i, rx, results).await;
        });
    }

    {
        let rx = rx.clone();
        let results = results.clone();
        tokio::spawn(async move {
            metrics_worker::main(rx, results).await;
        });
    }

    let state = AppState {
        work_queue: tx,
        results,
    };

    let governor_conf = GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_second(10)
        .burst_size(10)
        .finish()
        .expect("failed to build rate limit config");

    let execute_router = Router::new()
        .route("/api/v1/execute", post(crate::handlers::execute_code))
        .with_state(state.clone())
        .layer(GovernorLayer::new(governor_conf));

    let app = Router::new()
        .merge(execute_router)
        .route(
            "/api/v1/stream/{job_id}",
            get(crate::handlers::stream_results),
        )
        .route(
            "/api/v1/results/{job_id}",
            get(crate::handlers::get_results),
        )
        .route("/api/v1/version", get(crate::handlers::get_version))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO))
                .on_failure(trace::DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(CorsLayer::permissive())
        .fallback_service(ServeDir::new("../web/public"));

    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    info!("Listening on http://localhost:{port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
