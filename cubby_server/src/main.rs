#![doc = include_str!("../../README.md")]

mod config;
mod managers;

mod api;
mod utils;

use std::net::{IpAddr, SocketAddr};

use config::PROGRAM_CONFIG;
#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;
#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[global_allocator]
/// Sets Jemalloc as the default global allocator for better performance when
/// using polars
static GLOBAL: Jemalloc = Jemalloc;

use axum::{
    routing::{get, post},
    Router,
};
use tracing_subscriber::filter::LevelFilter;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::fmt()
        .with_max_level(match PROGRAM_CONFIG.log_level {
            0 => LevelFilter::ERROR,
            1 => LevelFilter::WARN,
            2 => LevelFilter::INFO,
            3 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE
        })
        .init();
    utils::setup_dataframes();
    // Create basic app
    let app = Router::new()
        .route(
            "/client/v3/register",
            post(api::client::accounts::register::endpoint),
        )
        .route(
            "/client/v3/register/available",
            get(api::client::accounts::get_username_availability::endpoint),
        )
        .with_state(managers::dataframes::DataframeManager::new());
    // Create listener
    let socket_addr =
        SocketAddr::new(IpAddr::from([0, 0, 0, 0]), PROGRAM_CONFIG.port);

    let listener = tokio::net::TcpListener::bind(socket_addr)
        .await
        .expect("Failed to start listener");
    axum::serve(listener, app).await.expect("Failed to serve axum app");
}
