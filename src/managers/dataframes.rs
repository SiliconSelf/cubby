//! Manages the dataframes used by the program

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use moka::future::Cache;
use once_cell::sync::Lazy;
use polars::prelude::*;

use crate::config::PROGRAM_CONFIG;

/// This template exists to have a small DataFrame that can be easily cloned to a new file
/// instead of evaluating a new one every time we need to make a file.
///
/// The performance savings from this are likely very minimal, but a server could find itself
/// creating many new files in situations such as an influx of users.
static TEMPLATE_FRAME: Lazy<Vec<u8>> = Lazy::new(|| {
    let mut df = df!(
        "foo" => &[1,2,3],
        "bar" => &[1,2,3]
    ).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    ParquetWriter::new(&mut buffer).finish(&mut df).unwrap();
    buffer
});

/// Manages the in-use DataFrames.
///
/// This struct should only exist once as part of the state being managed by the axum router
#[derive(Clone)]
pub(crate) struct DataframeManager {
    cache: Cache<PathBuf, Arc<RwLock<DataFrame>>>
}

impl DataframeManager {
    /// Create a new DataframeManager
    pub(crate) fn new() -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_millis(PROGRAM_CONFIG.cache_ttl))
            .time_to_idle(Duration::from_millis(PROGRAM_CONFIG.cache_tti))
            .eviction_listener(|k, v, _r| {

                write_dataframe_file(k, v)
            })
            .build();
        Self {
            cache
        }
    }
    /// Retrieve a LazyFrame from the cache, inserting it into the cache if it does not already
    /// exist.
    pub(crate) async fn get<P: Into<PathBuf>>(&self, path: P) -> Arc<RwLock<DataFrame>> {
        let key: PathBuf = path.into();
        self.cache.get_with_by_ref(&key, open_dataframe_file(key.as_path())).await
    }
}

/// Open an existing dataframe file, creating it if it doesn't already exist
///
/// # Panics
///
/// This function will panic if there is an issue creating a parquet file that doesn't already
/// exist. Removal of this panic is planned for the future.
async fn open_dataframe_file(path: &Path) -> Arc<RwLock<DataFrame>> {
    if !path.is_file() {
        let mut file = File::create(&path)
            .expect("Failed to create new parquet file");
        file.write_all(&TEMPLATE_FRAME)
            .expect("Failed to write template to new parquet file");
    }
    let mut file = File::open(path).unwrap();
    Arc::new(RwLock::new(ParquetReader::new(&mut file).finish().unwrap()))
}

/// Write a LazyFrame to disk
///
/// This is invoked as the eviction listener in the cache and probably shouldn't be used
/// anywhere else.
///
/// # Panics
///
/// This function will panic if anything at all goes wrong. It needs to be made more reliable later.
fn write_dataframe_file(path: Arc<PathBuf>, frame: Arc<RwLock<DataFrame>>) {
    let path = path.as_path();
    let mut file_handle = File::create(path).unwrap();
    let mut frame_handle = frame.write().unwrap();
    ParquetWriter::new(&mut file_handle).finish(&mut frame_handle).unwrap();
}