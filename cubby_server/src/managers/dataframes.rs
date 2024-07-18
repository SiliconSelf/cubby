//! Manages the dataframes used by the program
//!
//! This module honestly sucks and should be remade entirely in the future.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    fs::File,
    future::IntoFuture,
    io::Write,
    path::PathBuf,
    time::Duration,
};

use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError, Sender};
use once_cell::sync::Lazy;
use polars::prelude::*;
use tokio::sync::oneshot;
use tracing::{debug, error, instrument, trace};

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

trait Message {}

pub(crate) struct DataFrameManager {
    tx: Sender<Box<dyn Message>>,
    rx: Receiver<Box<dyn Message>>
}

impl DataFrameManager {
    pub(crate) fn new() -> Self {
        let (_thread_tx, manager_rx) = unbounded::<Box<dyn Message>>();
        let (manager_tx, _thread_rx) = unbounded::<Box<dyn Message>>();
        tokio::spawn(async move { todo!(); });
        Self {
            tx: manager_tx,
            rx: manager_rx
        }
    }
}

trait Receive<T> {
    fn handle(message: T);
}

#[derive(Debug)]
struct GetLockMessage<P> where P: Into<PathBuf> {
    path: P
}

impl<P> Receive<GetLockMessage<P>> for DataFrameManager where P: Into<PathBuf> + Debug {
    #[instrument(level = "trace")]
    fn handle(message: GetLockMessage<P>) {
        let _path: PathBuf = message.path.into();
        todo!();
    }
}

/// A wrapper around a given `LazyFrame`.
///
/// This struct has a custom Drop implementation that will send the current
/// contents back to the `DataframeManager` that created it to write changes to
/// disk.
pub(crate) struct ManagedLazyFrame {
    /// The internal `LazyFrame`
    frame: LazyFrame,
    /// The lock on the file, this is just here to prevent any other writers
    /// from getting a lock on the file while this is in use
    _file_lock: FileLock,
    /// The transmitter for sending the internal `frame` back to the manager
    tx: Sender<LazyFrame>,
}

impl ManagedLazyFrame {
    /// Run a closure taking the internal `LazyFrame` as an argument, replacing
    /// the internal frame with its result
    ///
    /// This function exists because pretty much all of the methods for
    /// `DataFrame` an`LazyFrame`me have the `fn x(self, ..) -> Self` function
    /// signature pattern, meaning it's basically impossible to use references
    /// in any productive way and any data processing at endpoints would require
    /// a large number of clones and a convoluted mess of channels. With this
    /// approach, we bring the function to the data because we can't bring the
    /// data to the function.
    pub(crate) fn apply<F: FnOnce(LazyFrame) -> LazyFrame>(mut self, closure: F) {
        self.frame = closure(self.frame.clone());
    }
}

impl Drop for ManagedLazyFrame {
    #[instrument(level = "trace", skip(self))]
    fn drop(&mut self) {
        trace!("Attempting to send LazyFrame back to manager during drop");
        self.tx
            .send(self.frame.clone());
    }
}