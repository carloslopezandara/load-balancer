/// HTTP response utilities for consistent API responses
pub mod http_utils;

/// All JSON models used across the application
pub mod models;

pub use http_utils::{create_error_response, create_json_response};
pub use models::*; // Export all models for easy access