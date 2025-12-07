/// HTTP Utility Functions for Admin API
/// 
/// This module provides reusable functions for creating consistent HTTP responses
/// across all admin endpoints.

use hyper::{Response, StatusCode};
use hyper::body::Bytes;
use http_body_util::{Full, BodyExt};
use serde::Serialize;

use crate::ResponseBody;
use super::models::ErrorResponse;

/// Creates a standardized JSON error response
/// 
/// This function builds consistent error responses across all admin endpoints
/// with appropriate HTTP status codes and detailed error information.
/// 
/// # Arguments
/// * `status` - HTTP status code for the error (e.g., BAD_REQUEST, NOT_FOUND)
/// * `error_msg` - Brief error message
/// * `details` - Optional detailed error information for debugging
/// 
/// # Returns
/// * Complete HTTP response with JSON error body
/// 
/// # Example
/// ```rust
/// let response = create_error_response(
///     StatusCode::BAD_REQUEST,
///     "Invalid JSON format", 
///     Some("JSON parse error: missing field")
/// );
/// ```
pub fn create_error_response(
    status: StatusCode, 
    error_msg: &str, 
    details: Option<&str>
) -> Response<ResponseBody> {
    let error_response = ErrorResponse {
        error: error_msg.to_string(),
        details: details.map(|d| d.to_string()),
    };
    
    let json = serde_json::to_string(&error_response)
        .unwrap_or_else(|_| r#"{"error": "Internal serialization error"}"#.to_string());
    
    let body = Full::new(Bytes::from(json))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed();
        
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body)
        .expect("Failed to build error response")
}

/// Creates a standardized JSON success response
/// 
/// This function serializes any serializable data structure into a JSON response
/// with appropriate headers and status codes.
/// 
/// # Arguments
/// * `data` - Any data structure that implements Serialize
/// 
/// # Returns
/// * Complete HTTP response with JSON body and 200 OK status
/// 
/// # Example
/// ```rust
/// let response_data = StrategyResponse {
///     current_strategy: "round_robin".to_string()
/// };
/// let response = create_json_response(&response_data);
/// ```
pub fn create_json_response<T: Serialize>(data: &T) -> Response<ResponseBody> {
    let json = serde_json::to_string(data)
        .unwrap_or_else(|_| r#"{"error": "Internal serialization error"}"#.to_string());
    
    let body = Full::new(Bytes::from(json))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed();
        
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(body)
        .expect("Failed to build JSON response")
}