//! Code related to the username availability checking endpoint.
//! 
//! [Spec](https://spec.matrix.org/latest/client-server-api/#get_matrixclientv3registeravailable)

use axum::extract::State;
use cubby_lib::{RumaExtractor, RumaResponder};
use polars::lazy::dsl::{col, lit};
use ruma::api::client::account::get_username_availability::v3::{Request, Response};

use crate::managers::dataframes::DataframeManager;

pub(crate) async fn endpoint(
    State(frames): State<DataframeManager>,
    RumaExtractor(req): RumaExtractor<Request>
) -> RumaResponder<Response> {
    let query = frames
        .get_lazy("users.parquet")
        .await
        .select(&[col("username")])
        .filter(col("username").eq(lit(req.username)))
        // .filter((col("username") == req.username.as_str().into()).into())
        .collect()
        .unwrap();
    println!("{query:?}");
    todo!();
}