// Cross-platform system monitoring for the dashboard and system pages.
//
// A single `SysMonitor` is kept alive in the app state so CPU and network
// deltas are measured between successive polls. Everything here is derived
// from the `sysinfo` crate, which works on macOS, Linux and Windows.
//
// The serializable payloads returned to the browser live in `dto`; the polling
// logic that produces them lives in `monitor`.

mod dto;
mod monitor;

pub use monitor::SysMonitor;
