use std::sync::Arc;

use polars::{datatypes::{DataType, RevMapping}, df, lazy::{dsl::col, frame::IntoLazy}};

use crate::managers::dataframes::DataframeManager;

pub(crate) async fn setup_dataframes() {
    let manager = DataframeManager::new();
    // Create users.parquet
    let users = df!("username" => ["cubby"])
        .unwrap();
    let users = users.lazy()
        .select([
            col("*").exclude(["username"]),
            col("username")
                .cast(DataType::Categorical(None, polars::datatypes::CategoricalOrdering::Lexical))
        ])
        .collect()
        .unwrap();
    let (_, tx) = manager
        .get_write("users.parquet")
        .await;
    tx.send(users).unwrap();
}