//! Manages the dataframes used by the program
//!
//! This module honestly sucks and should be remade entirely in the future.

use std::path::PathBuf;

use crossbeam_channel::{unbounded, Sender};
use cubby_lib::file_manager::{FileLock, FileManager, Message, Receive};
use polars::prelude::*;
use tracing::{instrument, trace};

/// A message requesting that the file manager return a `LazyFrame` for the
/// given path
pub(crate) struct GetLazyFrame<P>(P)
where
    P: Into<PathBuf>;
impl<P> Message for GetLazyFrame<P>
where
    P: Into<PathBuf>,
{
    type Response = Result<LazyFrame, PolarsError>;
}
impl<P> Receive<GetLazyFrame<P>> for FileManager
where
    P: Into<PathBuf>,
{
    async fn handle(
        &self,
        message: GetLazyFrame<P>,
    ) -> <GetLazyFrame<P> as Message>::Response {
        let path = message.0.into();
        LazyFrame::scan_parquet(path, ScanArgsParquet::default())
    }
}

/// A message requesting that the file manager return a `ManagedLazyFrame` for
/// the given path
pub(crate) struct GetManagedLazyFrame<P>(P)
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
        let lock = self.lock(path).await;
        ManagedLazyFrame::new(lock)
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
    /// The lock on the file underneath this `LazyFrame`. Once dropped, this
    /// will unlock the file for other threads to access it.
    _lock: FileLock,
    /// The transmitter for sending the internal `frame` back to the manager
    tx: Sender<LazyFrame>,
}

impl ManagedLazyFrame {
    /// Create a new `ManagedLazyFrame`
    pub(crate) fn new(lock: FileLock) -> Self {
        let (tx, rx) = unbounded::<LazyFrame>();
        tokio::spawn(async move {
            let Ok(received) = rx.recv() else {
                panic!("ManagedLazyFrame channel disconnected before drop");
            };
            let _frame = received.collect();
            // TODO: Merge frame here
        });
        Self {
            frame: LazyFrame::scan_parquet(
                lock.get_path_owned(),
                ScanArgsParquet::default(),
            )
            .expect("Failed to scan parquet file"),
            _lock: lock,
            tx,
        }
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
    /// `LazyFrame` from this closure and replace it with an empty one.
    /// Because of this, this pattern should not be depended on as a safety
    /// feature.
    // This function currently has an underscore prefix because it is not being
    // used yet. It will be used in the near future once frames need to start
    // being modified and have those changed written to disk.
    pub(crate) fn _apply<F: FnOnce(LazyFrame) -> LazyFrame>(
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

/// Functionality required for managing the parquet files used by the cubby
/// server
pub(crate) trait ParquetManager<P> {
    /// Get an unmanaged `LazyFrame`. If data needs to be mutated in a way that
    /// is written to persistent storage, `get_managed_lazyframe` should be used
    /// instead.
    async fn get_lazyframe(&self, path: P) -> Result<LazyFrame, PolarsError>;
    /// Get a managed `LazyFrame`. When dropped, any changes made to the
    /// internal `LazyFrame` via the `apply()` method will be written to disk.
    /// If data should not be written to disk when the `LazyFrame` is dropped,
    /// `get_lazyframe` should be used instead.
    async fn get_managed_lazyframe(&self, path: P) -> ManagedLazyFrame;
}

impl<P> ParquetManager<P> for FileManager
where
    P: Into<PathBuf>,
{
    async fn get_lazyframe(&self, path: P) -> Result<LazyFrame, PolarsError> {
        self.handle(GetLazyFrame(path)).await
    }

    async fn get_managed_lazyframe(&self, path: P) -> ManagedLazyFrame {
        self.handle(GetManagedLazyFrame(path)).await
    }
}
