#![doc = include_str!("../README.md")]

mod config;
mod managers;

mod api;

use std::net::{IpAddr, SocketAddr};

use config::PROGRAM_CONFIG;
#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;
#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[global_allocator]
/// Sets Jemalloc as the default global allocator for better performance when
/// using polars
static GLOBAL: Jemalloc = Jemalloc;

use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    // Create basic app
    let app = Router::new()
        .with_state(managers::dataframes::DataframeManager::new())
        .route("/", get(|| async { "Hello, World!" }))
        .nest("/_matrix/", api::client::accounts::create_router());
    // Create listener
    let socket_addr =
        SocketAddr::new(IpAddr::from([0, 0, 0, 0]), PROGRAM_CONFIG.port);

    let listener = tokio::net::TcpListener::bind(socket_addr)
        .await
        .expect("Failed to start listener");
    axum::serve(listener, app).await.expect("Failed to serve axum app");
}
