//! Code related to the username availability checking endpoint.
//!
//! [Spec](https://spec.matrix.org/latest/client-server-api/#get_matrixclientv3registeravailable)

use axum::{extract::State, http::StatusCode};
use cubby_lib::{IntoMatrixError, RumaExtractor, RumaResponder};
use polars::lazy::dsl::{col, lit};
use ruma::api::{
    client::account::get_username_availability::v3::{Request, Response},
    error::{MatrixError, MatrixErrorBody},
};
use serde_json::json;

use crate::managers::dataframes::DataframeManager;

pub(crate) enum EndpointErrors {
    InUse,
}

impl IntoMatrixError for EndpointErrors {
    fn into_matrix_error(self) -> MatrixError {
        match self {
            EndpointErrors::InUse => MatrixError {
                status_code: StatusCode::BAD_REQUEST,
                body: MatrixErrorBody::Json(json!({
                    "errcode": "M_USER_IN_USE",
                    "error": "The requested username is already in use"
                })),
            },
        }
    }
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
