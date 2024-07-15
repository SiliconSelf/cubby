//! Manages the dataframes used by the program

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::File,
    io::Write,
    path::PathBuf,
    time::Duration,
};

use crossbeam_channel::{unbounded, RecvTimeoutError, Sender};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use polars::prelude::*;
use tokio::sync::oneshot;

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

static LOCK_MANAGER: Lazy<LockManager> = Lazy::new(|| LockManager::new());

struct LockManager {
    locks: HashMap<PathBuf, Mutex<()>>,
    manager_tx: Sender<(PathBuf, oneshot::Sender<FileLock>)>,
}

impl LockManager {
    async fn get_lock<P: Into<PathBuf>>(&self, path: P) -> FileLock {
        let (tx, rx) = oneshot::channel();
        self.manager_tx.send((path.into(), tx));
        rx.await.expect("Channel communication failed")
    }

    fn new() -> Self {
        // Create channels to the other thread
        let (manager_tx, manager_rx) =
            unbounded::<(PathBuf, oneshot::Sender<FileLock>)>();
        // Spawn the task
        tokio::spawn(async move {
            let (lock_tx, lock_rx) = unbounded::<PathBuf>();
            let mut locks: HashSet<PathBuf> = HashSet::new();
            let mut queue: HashMap<
                PathBuf,
                VecDeque<oneshot::Sender<FileLock>>,
            > = HashMap::new();
            loop {
                // Eagerly release all pending locks
                loop {
                    match lock_rx.recv_timeout(Duration::from_millis(1)) {
                        Err(RecvTimeoutError::Timeout) => {
                            // All the pending locks have been released, move on
                            // to issuing new ones
                            break;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            panic!("Lock Manager became disconnected");
                        }
                        // A lock has been released
                        Ok(m) => {
                            tracing::debug!("Dropping lock for {m:?}");
                            locks.remove(&m);
                        }
                    };
                }
                // Queue all pending requests for locks
                loop {
                    match manager_rx.recv_timeout(Duration::from_millis(1)) {
                        Err(RecvTimeoutError::Timeout) => {
                            // Nothing more to add to the queue
                            break;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            panic!("Lock Manager became disconnected");
                        }
                        // A lock has been requested
                        Ok((p, s)) => {
                            if let Some(q) = queue.get_mut(&p) {
                                q.push_back(s);
                            } else {
                                let mut new_queue = VecDeque::new();
                                new_queue.push_back(s);
                                queue.insert(p, new_queue);
                            }
                        }
                    };
                }
                // Assign new locks based on any demand
                if !queue.is_empty() {
                    for (k, v) in &mut queue {
                        // Skip ahead to the next file if this one is locked
                        if locks.contains(k) {
                            continue;
                        }
                        // Issue a new lock
                        tracing::debug!("Locking {k:?}");
                        locks.insert(k.clone());
                        let lock = FileLock {
                            path: k.to_owned(),
                            channel: lock_tx.clone(),
                        };
                        v.pop_front()
                            .expect("We already checked that this isn't empty")
                            .send(lock)
                            .expect("Failed to send new file lock");
                    }
                }
            }
        });
        Self {
            locks: HashMap::new(),
            manager_tx,
        }
    }
}

#[derive(Debug)]
struct FileLock {
    path: PathBuf,
    channel: Sender<PathBuf>,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        self.channel
            .send(self.path.clone())
            .expect("Failed to send message to unlock my path");
    }
}

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
    /// Paths provided to this function are relative to the configured
    /// `data_path`
    ///
    /// This function is intended for read-only access to parquet data. For
    /// write access, please use `get_write` to take advantage of extra sync
    /// protections.
    pub(crate) async fn get_lazy<P: Into<PathBuf>>(
        &self,
        path: P,
    ) -> LazyFrame {
        scan_file(path).await
    }

    /// Returns an eager `DataFrame` suitable for writing
    pub(crate) async fn get_write<P: Into<PathBuf>>(
        &self,
        path: P,
    ) -> (DataFrame, Sender<DataFrame>) {
        let mut key = PROGRAM_CONFIG.data_path.clone();
        key.push(path.into());
        let scan = scan_file(&key).await.collect().unwrap();
        let (tx, rx) = unbounded::<DataFrame>();
        tokio::spawn(async move {
            tracing::debug!("Task spawned");
            // Block until we receive the new data to write
            let Ok(mut value) = rx.recv() else {
                tracing::error!(
                    "Receiving {key:?} failed! Did the endpoint give up?"
                );
                return;
            };
            tracing::debug!("Data received");
            tracing::debug!("Received returned LazyFrame for {key:?}");
            // Mom said it's my turn on the Mutex
            let file_lock = LOCK_MANAGER.get_lock(&key).await;
            // Write new data to path
            let mut file = File::create(key).unwrap();
            ParquetWriter::new(&mut file).finish(&mut value).unwrap();
            tracing::debug!("Wrote the thing");
        });
        (scan, tx)
    }
}

/// Scan a parquet file on disk, creating it from the `TEMPLATE_FRAME` if it
/// does not already exist.
///
/// Paths provided to this function are relative to the configured
/// `PROGRAM_CONFIG.data_path`.
async fn scan_file<P: Into<PathBuf>>(path: P) -> LazyFrame {
    let mut key = PROGRAM_CONFIG.data_path.clone();
    key.push(path.into());
    tracing::debug!("Scanning parquet file {key:?}");
    if !key.is_file() {
        tracing::debug!("Creating new parquet file: {key:?}");
        let mut file =
            File::create(&key).expect("Failed to create new parquet file");
        file.write_all(&TEMPLATE_FRAME)
            .expect("Failed to write template to new parquet file");
    }
    let scan_args = ScanArgsParquet::default();

    LazyFrame::scan_parquet(key, scan_args)
        .expect("Failed to scan parquet file")
}
