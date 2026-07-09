// JSON request/response shapes for the web API.
//
// `types` holds the plain serializable structs exchanged with the browser;
// `build` turns categorized scan entries into the results payload.

mod build;
mod types;

pub use build::build_results;
pub use types::*;
