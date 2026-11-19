//! Config — loader dengan resolusi berlapis + cache moka + snapshot + simulasi dampak.

pub mod loader;
pub mod simulate;
pub mod validate;

pub use loader::{ConfigContext, ConfigLoader, ConfigSource, ResolvedConfig};
