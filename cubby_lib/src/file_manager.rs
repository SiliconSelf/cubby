//! File Manager
//!
//! This module provides tools for mitigating race conditions when accessing resources on disk.
//!
//! TODO: Write more docs about how this works.

use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use tokio::sync::oneshot;

/// A message to be sent to and handled by the ``FileManager``.
pub trait Message {
    /// The response type handling this message should return
    type Response;
}

/// A trait implemented by the ``FileManager`` per message type
pub trait Receive<M>
where
    M: Message,
{
    /// Logic for handling a message provided to the ``FileManager``.
    #[allow(async_fn_in_trait)]
    async fn handle(&self, message: M) -> M::Response;
}

/// Represents a lock on an individual file.
///
/// This lock implements custom drop logic, phoning home to the ``FileManager`` that issued it to
/// indicate that a new lock can be issued to another thread.
#[derive(Debug)]
pub struct FileLock {
    /// The path this lock represents
    path: PathBuf,
    /// The internal sender for phoning home
    tx: Sender<PathBuf>,
}

impl FileLock {
    /// Gets a reference to the path this lock represents
    #[must_use] pub fn get_path(&self) -> &Path {
        self.path.as_path()
    }
    /// Clone the internal path
    #[must_use] pub fn get_path_owned(&self) -> PathBuf {
        self.path.clone()
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        self.tx.send(self.path.clone()).expect("TODO: panic message");
    }
}

#[derive(Debug, Clone)]
/// A ``FileManager``. See module-level docs for more details.
pub struct FileManager {
    /// The channel transmitter for communication with the management thread.
    pub tx: Sender<(PathBuf, oneshot::Sender<FileLock>)>,
}

impl FileManager {
    /// Create a new ``FileManager``
    #[must_use] pub fn new() -> Self {
        let (manager_tx, thread_rx) = crossbeam_channel::unbounded();
        tokio::spawn(async move {
            file_manager_thread(&thread_rx);
        });
        Self {
            tx: manager_tx,
        }
    }
    /// Request a lock on a specific file.
    ///
    /// This function will block until the lock is achieved.
    ///
    /// # Panics
    ///
    /// This function will panic if any channels it uses become disconnected while the program is still running
    pub async fn lock(&self, path: PathBuf) -> FileLock {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send((path, tx))
            .expect("Channel became disconnected while requesting lock");
        rx
            .await
            .expect("Channel became disconnected while waiting for lock")
    }
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

/// The background thread to manage locks requested and freed by the program via the ``FileManager``.
fn file_manager_thread(
    lock_rx: &Receiver<(PathBuf, oneshot::Sender<FileLock>)>,
) {
    let mut locks: HashMap<PathBuf, AtomicBool> = HashMap::new();
    let mut queue: HashMap<PathBuf, VecDeque<oneshot::Sender<FileLock>>> =
        HashMap::new();
    let (unlock_tx, unlock_rx) = crossbeam_channel::unbounded::<PathBuf>();
    loop {
        // Free any finished locks
        loop {
            match unlock_rx.try_recv() {
                Err(TryRecvError::Disconnected) => {
                    panic!("Thread became disconnected from DataframeManager");
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Ok(path) => {
                    if let Some(lock) = locks.get_mut(&path) {
                        // We can mark the file as available
                        lock.store(false, Ordering::Relaxed);
                    } else {
                        // I'm not sure how this path would ever be reached, but
                        // can't hurt to cover it
                        locks.insert(path, AtomicBool::new(false));
                        continue;
                    };
                    // Restart the loop to free locks as eagerly as possible
                    continue;
                }
            }
        }
        // Add new requests to the queue
        loop {
            match lock_rx.try_recv() {
                Err(TryRecvError::Disconnected) => {
                    panic!("Thread became disconnected from manager");
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Ok((path, tx)) => {
                    if let Some(handle) = queue.get_mut(&path) {
                        handle.push_back(tx);
                    } else {
                        let mut new_queue = VecDeque::new();
                        new_queue.push_back(tx);
                        queue.insert(path, new_queue);
                    }
                    continue;
                }
            }
        }
        // Issue any new locks once all pending locks are freed
        for (k, v) in &mut queue {
            // Skip to the next iteration if the requested file is locked
            if let Some(lock) = locks.get(k) {
                if lock.load(Ordering::Relaxed) {
                    continue;
                }
                lock.store(true, Ordering::Relaxed);
            } else {
                // If the requested file is not in the map, add and lock it
                locks.insert(k.clone(), AtomicBool::new(true));
            }
            // Issue a new lock
            if let Some(tx) = v.pop_front() {
                tx.send(FileLock {
                    path: k.to_owned(),
                    tx: unlock_tx.clone(),
                })
                .expect("TODO: panic message");
            }
        }
    }
}
