#![doc = include_str!("../README.md")]

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;
#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use axum::{
    routing::get,
    Router
};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    // Create basic app
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }));
    // Create listener
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await
        .expect("Failed to start listener");
    axum::serve(listener, app).await
        .expect("Failed to serve axum app");
}
