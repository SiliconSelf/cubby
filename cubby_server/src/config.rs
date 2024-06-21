//! Global program configuration

use std::path::PathBuf;

use once_cell::sync::Lazy;

/// The single source of truth for global homeserver configuration
pub(crate) static PROGRAM_CONFIG: Lazy<Config> = Lazy::new(|| {
    // TODO: Load this from the environment
    Config::default()
});

/// Represents an instance of the global program configuration
#[derive(Debug)]
pub(crate) struct Config {
    /// If the server should be allowed to federate with other servers.
    ///
    /// Defaults to `false`
    pub(crate) _enable_federation: bool,
    /// What port the server should listen on.
    /// 
    /// Defaults to `3000`
    pub(crate) port: u16,
    /// The time in milliseconds a `DataFrame` should be allowed to be held in memory before being
    /// dropped from the cache and written to disk. This time is measured from when the `DataFrame`
    /// is added to the cache.
    ///
    /// Extending this period may increase server RAM usage, but also provide faster access to more
    /// chats at a given time.
    ///
    /// This value defaults to `10_000`
    pub(crate) cache_ttl: u64,
    /// The time in milliseconds a `DataFrame` should be allowed to idle in memory before being
    /// dropped from the cache and written to disk. This time is measured from when the `DataFrame`
    /// is last accessed from the cache.
    ///
    /// Extending this period may increase server RAM usage, but also provide faster access to more
    /// chats at a given time.
    ///
    /// This value defaults to `1_000`
    pub(crate) cache_tti: u64,
    /// Where to store the parquet files for the homeserver
    /// 
    /// This defaults to a temporary directory that will NOT be deleted when the server shuts down.
    /// This is probably undesirable for your use case. You should change this directory.
    pub(crate) data_path: PathBuf,
    /// Where to store media that gets uploaded to the server.
    /// 
    /// This is optional and will default to `data_path/media/` if unset.
    pub(crate) media_path: PathBuf,
    /// How long generated devide ids should be.
    /// 
    /// You proably don't need to change this. Defaults to 16.
    pub(crate) device_id_length: u8,
    /// Is registration allowed on this server
    /// 
    /// You almost certainly do not want this to be enabled. Turning this on will turn your server
    /// into a bot farm, completely exposed to the cyber wilderness for anything with cURL
    /// installed to make an account and do what it pleases with your server.
    /// 
    /// It defaults to false, obviously.
    pub(crate) allow_registration: bool
}

impl Default for Config {
    fn default() -> Self {
        let tempdir = tempdir::TempDir::new("cubby").unwrap().into_path();
        let mut media_tempdir = tempdir.clone();
        media_tempdir.push("/media");
        Self {
            _enable_federation: false,
            port: 3000,
            cache_ttl: 10_000,
            cache_tti: 1_000,
            data_path: tempdir,
            media_path: media_tempdir,
            device_id_length: 16,
            #[cfg(debug_assertions)]
            allow_registration: true,
            #[cfg(not(debug_assertions))]
            allow_registration: false
        }
    }
}
