pub mod github;
pub mod gucs;
pub mod postgres;
pub mod telemetry;

#[cfg(feature = "fixtures")]
pub mod fixtures;

// We need to re-export the dependencies below, because they're used by our public macros.
// This lets consumers of the macros use them without needing to also install these dependencies.
pub use pgrx;
pub use serde_json;
