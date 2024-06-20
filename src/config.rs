//! Global program configuration

use once_cell::sync::Lazy;

/// The single source of truth for global homeserver configuration
pub(crate) static PROGRAM_CONFIG: Lazy<Config> = Lazy::new(|| {
    // TODO: Load this from the environment
    Config::default()
});

/// Represents an instance of the global program configuration
pub(crate) struct Config {
    /// If the server should be allowed to federate with other servers.
    ///
    /// Defaults to `false`
    pub(crate) enable_federation: bool,
    /// What port the server should listen on.
    /// 
    /// Defaults to `3000`
    pub(crate) port: u16,
    /// The time in milliseconds a DataFrame should be allowed to be held in memory before being
    /// dropped from the cache and written to disk. This time is measured from when the DataFrame
    /// is added to the cache.
    ///
    /// Extending this period may increase server RAM usage, but also provide faster access to more
    /// chats at a given time.
    ///
    /// This value defaults to `10_000`
    pub(crate) cache_ttl: u64,
    /// The time in milliseconds a DataFrame should be allowed to idle in memory before being
    /// dropped from the cache and written to disk. This time is measured from when the DataFrame is
    /// last accessed from the cache.
    ///
    /// Extending this period may increase server RAM usage, but also provide faster access to more
    /// chats at a given time.
    ///
    /// This value defaults to `1_000`
    pub(crate) cache_tti: u64
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_federation: false,
            port: 3000,
            cache_ttl: 10_000,
            cache_tti: 1_000
        }
    }
}
