pub mod constants;
pub mod logs;
pub mod telemetry;
pub mod testing;

#[cfg(feature = "fixtures")]
pub mod fixtures;

// We need to re-export the dependencies below, because they're used by our public macros.
// This lets consumers of the macros use them without needing to also install these dependencies.
pub use pgrx;
pub use serde_json;

// We re-export sqlx in test mode for use in the integration test suites in our extensions.
// This ensures the consumers are using the same version of sqlx with the same features.
#[cfg(feature = "fixtures")]
pub use sqlx;
