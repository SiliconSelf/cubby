//! Various utilities used throughout the program.
//!
//! Most if not all of what exists in this module should only be used during
//! development. Anything actually used in runtime in multiple places probably
//! belongs in `cubby_lib`

use polars::{
    datatypes::DataType,
    df,
    lazy::{dsl::col, frame::IntoLazy},
};

use crate::managers::dataframes::DataframeManager;

/// Creates initial dataframes that are required to exist when the program first
/// starts
///
/// This function should be removed eventually in favor of something smarter
pub(crate) fn setup_dataframes() {
    let manager = DataframeManager::new();
    // Create users.parquet
    let users = df!("username" => ["cubby"]).unwrap();
    let users = users
        .lazy()
        .select([
            col("*").exclude(["username"]),
            col("username").cast(DataType::Categorical(
                None,
                polars::datatypes::CategoricalOrdering::Lexical,
            )),
        ])
        .collect()
        .expect("Creating the users dataframe failed");
    let (_, tx) = manager.get_write("users.parquet");
    tx.send(users).expect("Sending the dataframe to be written failed");
}
