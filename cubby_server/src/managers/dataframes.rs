//! Manages the dataframes used by the program

use std::{
    collections::HashMap, fs::File, io::Write, path::PathBuf
};

use crossbeam_channel::{unbounded, Sender};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use polars::prelude::*;

use crate::config::PROGRAM_CONFIG;

/// This template exists to have a small `DataFrame` that can be easily cloned
/// to a new file instead of evaluating a new one every time we need to make a
/// file.
///
/// The performance savings from this are likely very minimal, but a server
/// could find itself creating many new files in situations such as an influx of
/// users.
static TEMPLATE_FRAME: Lazy<Vec<u8>> = Lazy::new(|| {
    let mut df = df!("0" => [0]).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    ParquetWriter::new(&mut buffer).finish(&mut df).unwrap();
    buffer
});

// This is fucking heinous
static LOCKS: Lazy<Arc<RwLock<HashMap<PathBuf, Mutex<()>>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

/// Manages the in-use `DataFrames`.
///
/// This struct should only exist once as part of the state being managed by the
/// axum router
#[derive(Clone)]
pub(crate) struct DataframeManager;

impl DataframeManager {
    /// Create a new `DataframeManager`
    pub(crate) fn new() -> Self {
        Self {}
    }
    /// Retrieve a `LazyFrame` from the cache, inserting it into the cache if it
    /// does not already exist.
    ///
    /// Paths provided to this function are relative to the configured data_path
    /// 
    /// This function is intended for read-only access to parquet data. For write access,
    /// please use get_write to take advantage of extra sync protections.
    pub(crate) async fn get_lazy<P: Into<PathBuf>>(
        &self,
        path: P,
    ) -> LazyFrame {
        scan_file(path).await
    }
    /// Returns an eager DataFrame suitable for writing
    pub(crate) async fn get_write<P: Into<PathBuf>>(&self, path: P) -> (DataFrame, Sender<DataFrame>) {
        let mut key = PROGRAM_CONFIG.data_path.clone();
        key.push(path.into());
        let scan = scan_file(&key)
            .await
            .collect()
            .unwrap();
        let (tx, rx) = unbounded::<DataFrame>();
        tokio::spawn(async move {
            tracing::info!("Task spawned");
            // Block until we receive the new data to write
            let Ok(mut value) = rx.recv() else {
                tracing::warn!("Receiving {key:?} failed! Did the endpoint give up?");
                return
            };
            tracing::info!("Data received");
            tracing::info!("Received returned LazyFrame for {key:?}");
            // Mom said it's my turn on the Mutex
            let mut handle = LOCKS.write();
            let _lock = match handle.get(&key) {
                Some(m) => { m.lock() },
                None => {
                    handle.insert(key.clone(), Mutex::new(()));
                    handle.get(&key).unwrap().lock()
                }
            };
            // Write new data to path
            let mut file = File::create(key).unwrap();
            ParquetWriter::new(&mut file).finish(&mut value).unwrap();
            tracing::info!("Wrote the thing");
        });
        (scan, tx)
    }
}

/// Scan a parquet file on disk, creating it from the TEMPLATE_FRAME if it does not already exist.
/// 
/// Paths provided to this function are relative to the configured PROGRAM_CONFIG.data_path.
async fn scan_file<P: Into<PathBuf>>(path: P) -> LazyFrame {
    let mut key = PROGRAM_CONFIG.data_path.clone();
    key.push(path.into());
    tracing::info!("Scanning parquet file {key:?}");
    if !key.is_file() {
        tracing::info!("Creating new parquet file: {key:?}");
        let mut file = File::create(&key)
            .expect("Failed to create new parquet file");
        file.write_all(&TEMPLATE_FRAME)
            .expect("Failed to write template to new parquet file");
    }
    let scan_args = ScanArgsParquet::default();
    let lf = LazyFrame::scan_parquet(key, scan_args)
        .expect("Failed to scan parquet file");
    lf
}