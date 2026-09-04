//! Headless orchestration: the engine's public API.
//!
//! Everything the UI can do is expressible here, which is what keeps the whole
//! product testable in CI without opening a window.

/// The edit compiler: a contour change becomes a new rest state.
pub mod couture;
/// The shared demo scene, used by goldens, benchmarks and the app.
pub mod demo;
/// The document resolved into geometry, and the door the interface uses.
pub mod draft;
/// The determinism golden.
pub mod golden;
/// The client-facing editing session.
pub mod session;
/// The simulation thread and its message contract.
pub mod sync;
