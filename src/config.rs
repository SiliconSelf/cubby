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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_federation: false,
            port: 3000,
        }
    }
}
