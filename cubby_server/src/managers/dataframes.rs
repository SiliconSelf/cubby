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
    ParquetWriter::new(&mut buffer)
        .finish(&mut df)
        .expect("Failed to create Parquet Writer for TEMPLATE_FRAME");
    buffer
});

/// This static exists to manage all of the file locks held by the program at
/// any given time.
///
/// See the documentation for ``LockManager`` for more details.
static LOCK_MANAGER: Lazy<LockManager> = Lazy::new(LockManager::new);

/// Eagerly manages a series of locks under its control.
///
/// The primary method of interaction with the `LockManager` is through the
/// ``get_lock()`` method where, when given a file path, the manager will return
/// a future that can be awaited until a lock is achieved on the file. Once the
/// `FileLock` is dropped, it will send a message back to the manager saying it
/// has been dropped and to unlock the path it was locking for the next request
/// in line.
///
/// Requests for file locks are in a first-come-first-serve basis. When freeing
/// locks, the manager will eagerly process all pending `FileLock` drop messages
/// until none remain in its channel. It will then iterate through the entire
/// queue of requests for locks and issue as many as possible until all
/// requested files are locked or until there are no more requests in the
/// queue..
struct LockManager {
    /// The internal transmitter for sending requests for locks to the detached
    /// thread.
    manager_tx: Sender<(PathBuf, oneshot::Sender<FileLock>)>,
}

impl LockManager {
    /// Request a lock on a specific file
    async fn get_lock<P: Into<PathBuf>>(&self, path: P) -> FileLock {
        let (tx, rx) = oneshot::channel();
        self.manager_tx
            .send((path.into(), tx))
            .expect("Channel communication failed");
        rx.await.expect("Channel communication failed")
    }

    /// Create a new `LockManager`. Realistically, this should only be called
    /// once while creating the stati`LOCK_MANAGER`ER.
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
            manager_tx,
        }
    }
}

/// Represents a lock on a specific file
///
/// When dropped, this struct will send a message back to the lock manager that
/// issued it.
#[derive(Debug)]
struct FileLock {
    /// The file path is lock represents
    path: PathBuf,
    /// The channel back to the sender that issued the lock
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
    #[allow(clippy::unused_self)]
    pub(crate) fn get_lazy<P: Into<PathBuf>>(&self, path: P) -> LazyFrame {
        scan_file(path)
    }

    /// Returns an eager `DataFrame` suitable for writing
    #[allow(clippy::unused_self)]
    pub(crate) fn get_write<P: Into<PathBuf>>(
        &self,
        path: P,
    ) -> (DataFrame, Sender<DataFrame>) {
        let mut key = PROGRAM_CONFIG.data_path.clone();
        key.push(path.into());
        let scan = scan_file(&key).collect().expect("Failed to scan file");
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
            let _file_lock = LOCK_MANAGER.get_lock(&key).await;
            // Write new data to path
            let mut file = File::create(key).expect("Failed to create file");
            ParquetWriter::new(&mut file)
                .finish(&mut value)
                .expect("Failed to write file");
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
fn scan_file<P: Into<PathBuf>>(path: P) -> LazyFrame {
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
