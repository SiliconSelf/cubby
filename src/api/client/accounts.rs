//! Accounts related endpoints

use axum::{routing::post, Router};

use ruma::api::client::account::register;

use crate::api::{RumaExtractor, RumaResponder};

pub(crate) async fn register(req: RumaExtractor<register::v3::Request>) -> RumaResponder<register::v3::Response> {
    todo!();
}

// Creates a router for account endpoints to mount to the main application
pub(crate) fn create_router() -> Router {
    Router::new()
        .route("/client/v3/register", post(register))
}