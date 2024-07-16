//! Code related to the username availability checking endpoint.
//!
//! [Spec](https://spec.matrix.org/latest/client-server-api/#get_matrixclientv3registeravailable)

use axum::extract::State;
use cubby_lib::{CubbyResponder, RumaExtractor};
use cubby_macros::IntoMatrixError;
use polars::lazy::dsl::{col, lit};
use ruma::api::client::account::get_username_availability::v3::{
    Request, Response,
};

use crate::managers::dataframes::DataframeManager;

/// All possible errors that can be returned from the endpoint
#[derive(IntoMatrixError)]
pub(crate) enum EndpointErrors {
    /// The requested username was already in use
    #[matrix_error(
        BAD_REQUEST,
        "M_USER_IN_USE",
        "The requested username is already in use"
    )]
    InUse,
    /// The requested username was invalid
    #[matrix_error(
        BAD_REQUEST,
        "M_INVALID_USERNAME",
        "The requested username is not allowed by the homeserver"
    )]
    _InvalidUsername,
    /// The request username is in the namespace of an appservice
    #[matrix_error(
        BAD_REQUEST,
        "M_EXCLUSIVE",
        "The requested username is in the exclusive namespace of an appservice"
    )]
    _Exclusive,
    /// There was an error running the polars query
    #[matrix_error(
        INTERNAL_SERVER_ERROR,
        "M_INTERNAL_SERVER_ERROR",
        "There was a problem executing the polars query"
    )]
    PolarsError,
}

pub(crate) async fn endpoint(
    State(frames): State<DataframeManager>,
    RumaExtractor(req): RumaExtractor<Request>,
) -> CubbyResponder<Response, EndpointErrors> {
    let Ok(query) = frames
        .get_lazy("users.parquet")
        .select(&[col("username")])
        .filter(col("username").eq(lit(req.username)))
        .collect()
    else {
        return CubbyResponder::MatrixError(EndpointErrors::PolarsError);
    };
    if query
        .column("username")
        .expect("We already checked that this exists")
        .is_empty()
    {
        CubbyResponder::Ruma(Response::new(true))
    } else {
        CubbyResponder::MatrixError(EndpointErrors::InUse)
    }
}
