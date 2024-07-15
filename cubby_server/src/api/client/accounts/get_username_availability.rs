//! Code related to the username availability checking endpoint.
//!
//! [Spec](https://spec.matrix.org/latest/client-server-api/#get_matrixclientv3registeravailable)

use axum::extract::State;
use cubby_lib::{RumaExtractor, RumaResponder};
use cubby_macros::IntoMatrixError;
use polars::lazy::dsl::{col, lit};
use ruma::api::
    client::account::get_username_availability::v3::{Request, Response}
;
use serde_json::json;

use crate::managers::dataframes::DataframeManager;

#[derive(IntoMatrixError)]
pub(crate) enum EndpointErrors {
    #[matrix_error(
        BAD_REQUEST,
        "M_USER_IN_USE",
        "The requested username is already in use"
    )]
    InUse,
    #[matrix_error(
        BAD_REQUEST,
        "M_INVALID_USERNAME",
        "The requested username is not allowed by the homeserver"
    )]
    _InvalidUsername,
    #[matrix_error(
        BAD_REQUEST,
        "M_EXCLUSIVE",
        "The requested username is in the exclusive namespace of an appservice"
    )]
    _Exclusive,
}

pub(crate) async fn endpoint(
    State(frames): State<DataframeManager>,
    RumaExtractor(req): RumaExtractor<Request>,
) -> RumaResponder<Response, EndpointErrors> {
    let query = frames
        .get_lazy("users.parquet")
        .await
        .select(&[col("username")])
        .filter(col("username").eq(lit(req.username)))
        .collect()
        .unwrap();
    if query.column("username").unwrap().is_empty() {
        RumaResponder::Ok(Response::new(true))
    } else {
        RumaResponder::Err(EndpointErrors::InUse)
    }
}
