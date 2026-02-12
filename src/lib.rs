//! Frame2 Photo Frame Server Library
//!
//! This library exposes the core application components for use in tests
//! and the main binary.

pub mod app;
pub mod models;
pub mod on_disk_photo;
pub mod test_helpers;

pub use app::{App, APIResult};
