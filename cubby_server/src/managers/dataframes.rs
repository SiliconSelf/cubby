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
    sync::Mutex,
    time::Duration,
};

use crossbeam_channel::{
    unbounded, Receiver, RecvError, RecvTimeoutError, Sender,
};
use cubby_lib::file_manager::{FileLock, FileManager, Message, Receive};
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

struct GetManagedLazyFrame<P>(P)
where
    P: Into<PathBuf>;

impl<P> Message for GetManagedLazyFrame<P>
where
    P: Into<PathBuf>,
{
    type Response = ManagedLazyFrame;
}

impl<P> Receive<GetManagedLazyFrame<P>> for FileManager
where
    P: Into<PathBuf>,
{
    async fn handle(
        &self,
        message: GetManagedLazyFrame<P>,
    ) -> ManagedLazyFrame {
        let path = message.0.into();
        let lock = self.get_lock(path).await;
        let frame = ManagedLazyFrame::new(lock);
        frame
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
    /// The lock on the file underneath this LazyFrame. Once dropped, this will
    /// unlock the file for other threads to access it.
    _lock: FileLock,
    /// The transmitter for sending the internal `frame` back to the manager
    tx: Sender<LazyFrame>,
}

impl ManagedLazyFrame {
    pub(crate) fn new(lock: FileLock) -> Self {
        todo!();
    }

    /// Run a closure taking the internal `LazyFrame` as an argument, replacing
    /// the internal frame with its result
    ///
    /// This function exists because pretty much all the methods for
    /// `DataFrame` an`LazyFrame`me have the `fn x(self, …) -> Self` function
    /// signature pattern, meaning it's basically impossible to use references
    /// in any productive way and any data processing at endpoints would require
    /// a large number of clones and a convoluted mess of channels. With this
    /// approach, we bring the function to the data because we can't bring the
    /// data to the function.
    ///
    /// It is theoretically (and trivially) possible to extract the internal
    /// LazyFrame from this closure and replace it with an empty one.
    /// Because of this, this pattern should not be depended on as a safety
    /// feature.
    pub(crate) fn apply<F: FnOnce(LazyFrame) -> LazyFrame>(
        mut self,
        closure: F,
    ) {
        self.frame = closure(self.frame.clone());
    }
}

impl Drop for ManagedLazyFrame {
    #[instrument(level = "trace", skip(self))]
    fn drop(&mut self) {
        trace!("Attempting to send LazyFrame back to manager during drop");
        self.tx.send(self.frame.clone()).expect("TODO: panic message");
    }
}
