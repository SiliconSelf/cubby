//! Various utilities used throughout the program.
//!
//! Most if not all of what exists in this module should only be used during
//! development. Anything actually used in runtime in multiple places probably
//! belongs in `cubby_lib`

use cubby_lib::FileManager;
use polars::{
    datatypes::DataType,
    df,
    lazy::{
        dsl::{col, cols},
        frame::IntoLazy,
    },
};

/// Creates a new parquet file from a provided template
///
/// # Arguments
/// - `$file_name`: the string literal of the path the new parquet file should
///   be written to.
/// - `$df`: the new dataframe to write at that path
/// - `$cat`: an array of string literals with the names of any columns that
///   should be cast to a categorical datatype for string interning before
///   writing them to disk.
macro_rules! parquet_file {
    ($file_name:literal, $df:expr) => {
        let frame = $df.expect("Created dataframe is invalid");
        let (_, tx) = FileManager::new().get_write($file_name);
        tx.send(frame).expect("Sending the dataframe to be written failed");
    };
    ($file_name:literal, $df:expr, $cat:tt) => {
        let frame = $df.expect("Created dataframe is invalid");
        let frame = frame
            .lazy()
            .select([
                col("*").exclude($cat),
                cols($cat).cast(DataType::Categorical(
                    None,
                    polars::datatypes::CategoricalOrdering::default(),
                )),
            ])
            .collect()
            .expect("Casting columns to categorical datatype failed");
        let (_, tx) = FileManager::new().get_write($file_name);
        tx.send(frame).expect("Sending the dataframe to be written failed");
    };
}

/// Creates initial dataframes that are required to exist when the program first
/// starts
///
/// This function should be removed eventually in favor of something smarter
pub(crate) fn setup_dataframes() {
    // parquet_file!("users.parquet", df!("username" => ["cubby"]),
    // ["username"]);
}
