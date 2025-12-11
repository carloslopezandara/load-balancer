/// HTTP Utility Functions

use hyper::{Response, StatusCode, Request};
use hyper::header::CONTENT_TYPE;
use hyper::body::Incoming;
use serde::{Serialize, de::DeserializeOwned};
use http_body_util::{Full, BodyExt};
use hyper::body::Bytes;

use crate::domain::{LoadBalancerError, Result};
use crate::ResponseBody;
use super::constants::headers;

/// Parse JSON body from HTTP request
/// 
/// This function handles all the steps of extracting and parsing a JSON body:
/// 1. Collects the body bytes from the request
/// 2. Converts to UTF-8 string
/// 3. Parses JSON into the target type
/// 
/// All errors are properly logged and converted to LoadBalancerError.
pub async fn parse_json_body<T: DeserializeOwned>(
    req: Request<Incoming>
) -> Result<T> {
    let body_bytes = http_body_util::BodyExt::collect(req.into_body()).await
        .map_err(|e| {
            let error = LoadBalancerError::from(e);
            tracing::error!("Failed to read request body: {}", error);
            error
        })?
        .to_bytes();

    let body_str = String::from_utf8(body_bytes.to_vec())
        .map_err(|e| {
            let error = LoadBalancerError::from(e);
            tracing::warn!("Invalid UTF-8 in request body: {}", error);
            error
        })?;

    serde_json::from_str(&body_str)
        .map_err(|e| {
            let error = LoadBalancerError::from(e);
            tracing::warn!("Invalid JSON format: {}", error);
            error
        })
}

/// Create error response from LoadBalancerError 
pub fn create_error_response(error: &LoadBalancerError) -> Response<ResponseBody> {
    let error_response = serde_json::json!({
        "error": error.status_code().canonical_reason().unwrap_or("Unknown Error"),
        "message": error.user_message(),
        "details": format!("{}", error),
        "status": error.status_code().as_u16()
    });

    create_json_response_with_status(&error_response, error.status_code())
        .unwrap_or_else(|e| {
            tracing::error!("Failed to serialize error response: {}", e);
            create_fallback_error_response()
        })
}

/// Create JSON response with status
pub fn create_json_response_with_status<T: Serialize>(
    data: &T, 
    status: StatusCode
) -> Result<Response<ResponseBody>> {
    let json = serde_json::to_string(data)
        .map_err(|e| LoadBalancerError::json_parsing("response serialization", e))?;

    let body = Full::new(Bytes::from(json))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed();

    let response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, headers::APPLICATION_JSON)
        .body(body)
        .map_err(|e| LoadBalancerError::internal(format!("Failed to build HTTP response: {}", e)))?;

    Ok(response)
}

/// Simple fallback error response that never panics
pub fn create_fallback_error_response() -> Response<ResponseBody> {
    // Try to create a nice JSON response first
    let json_body = r#"{"error":"Internal server error","message":"An unexpected error occurred"}"#;
    let body = Full::new(Bytes::from(json_body))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed();
    
    // Try with JSON content-type header
    if let Ok(response) = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(CONTENT_TYPE, headers::APPLICATION_JSON)
        .body(body) {
        return response;
    }
    
    // If that fails, try without custom headers
    let simple_body = Full::new(Bytes::from("Internal server error"))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed();
    
    if let Ok(response) = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(simple_body) {
        tracing::warn!("Fallback response created without JSON headers");
        return response;
    }
    
    // Absolute last resort - basic response with default status
    tracing::error!("CRITICAL: Cannot create normal HTTP response, using minimal fallback");
    Response::new(Full::new(Bytes::from("Error"))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        .boxed())
}