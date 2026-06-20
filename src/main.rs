mod db;
mod errors;
mod handlers;
mod models;
mod scheduler;
mod services;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::init_db;
use crate::services::AuctionService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "auction_lock_service=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:auction.db?mode=rwc".to_string());

    let pool = init_db(&database_url).await?;
    tracing::info!("Database initialized successfully");

    let service = Arc::new(AuctionService::new(pool.clone()));

    let scheduler_service = service.clone();
    tokio::spawn(async move {
        scheduler::start_expired_locks_cleanup(scheduler_service, 60).await;
    });
    tracing::info!("Expired locks cleanup scheduler started");

    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/products", post(handlers::create_product))
        .route("/products/:product_id", get(handlers::get_product))
        .route(
            "/products/room/:room_id",
            get(handlers::get_products_by_room),
        )
        .route(
            "/products/:product_id/locks",
            get(handlers::get_product_locks),
        )
        .route("/bids", post(handlers::place_bid))
        .route("/bids/:bid_id", get(handlers::get_bid))
        .route("/bids/:bid_id/confirm", post(handlers::confirm_purchase))
        .route("/bids/:bid_id/cancel", post(handlers::cancel_bid))
        .with_state(service);

    let addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse::<SocketAddr>()?;

    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
