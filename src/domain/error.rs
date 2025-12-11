/// Load Balancer Error Types
/// 
/// Centralized error handling with structured error types
/// that can be easily converted to HTTP responses.

use thiserror::Error;

use crate::{ResponseBody, utils::create_error_response};

/// Main error type for load balancer operations
/// 
/// This enum covers all possible error scenarios in the load balancer
#[derive(Error, Debug)]
pub enum LoadBalancerError {
    /// Worker connection or communication errors
    #[error("Worker connection failed: {worker_url}")]
    WorkerConnection {
        worker_url: String,
        #[source]
        source: hyper::Error,
    },

    /// HTTP client errors from hyper_util
    #[error("HTTP client error for worker: {worker_url}")]
    HttpClient {
        worker_url: String,
        #[source]
        source: hyper_util::client::legacy::Error,
    },

    /// Invalid load balancing strategy provided
    #[error("Invalid load balancing strategy: '{strategy}'. Valid strategies are: round_robin, least_connections")]
    InvalidStrategy { strategy: String },

    /// No workers available for load balancing
    #[error("No workers available for load balancing")]
    NoWorkersAvailable,

    /// Request parsing or validation errors
    #[error("Request validation failed: {message}")]
    RequestValidation { message: String },

    /// JSON parsing errors
    #[error("JSON parsing failed: {context}")]
    JsonParsing {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    /// HTTP body processing errors
    #[error("HTTP body processing failed: {context}")]
    HttpBodyProcessing {
        context: String,
        #[source]
        source: hyper::Error,
    },

    /// UTF-8 conversion errors
    #[error("UTF-8 conversion failed: {context}")]
    Utf8Conversion {
        context: String,
        #[source]
        source: std::string::FromUtf8Error,
    },

    /// Internal server errors
    #[error("Internal server error: {message}")]
    Internal { message: String },

    /// Resource not found errors
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },
}

/// Helper functions for creating specific error types
impl LoadBalancerError {
    /// Create a worker connection error
    pub fn worker_connection(worker_url: impl Into<String>, source: hyper::Error) -> Self {
        Self::WorkerConnection {
            worker_url: worker_url.into(),
            source,
        }
    }

    /// Create an HTTP client error
    pub fn http_client(worker_url: impl Into<String>, source: hyper_util::client::legacy::Error) -> Self {
        Self::HttpClient {
            worker_url: worker_url.into(),
            source,
        }
    }

    /// Create an invalid strategy error
    pub fn invalid_strategy(strategy: impl Into<String>) -> Self {
        Self::InvalidStrategy {
            strategy: strategy.into(),
        }
    }

    /// Create a request validation error
    pub fn request_validation(message: impl Into<String>) -> Self {
        Self::RequestValidation {
            message: message.into(),
        }
    }

    /// Create a JSON parsing error
    pub fn json_parsing(context: impl Into<String>, source: serde_json::Error) -> Self {
        Self::JsonParsing {
            context: context.into(),
            source,
        }
    }

    /// Create an HTTP body processing error
    pub fn http_body_processing(context: impl Into<String>, source: hyper::Error) -> Self {
        Self::HttpBodyProcessing {
            context: context.into(),
            source,
        }
    }

    /// Create a UTF-8 conversion error
    pub fn utf8_conversion(context: impl Into<String>, source: std::string::FromUtf8Error) -> Self {
        Self::Utf8Conversion {
            context: context.into(),
            source,
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            resource: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }
}

/// Convert LoadBalancerError to appropriate HTTP status code
impl LoadBalancerError {
    /// Get the appropriate HTTP status code for this error
    pub fn status_code(&self) -> hyper::StatusCode {
        use hyper::StatusCode;
        
        match self {
            LoadBalancerError::WorkerConnection { .. } => StatusCode::BAD_GATEWAY,
            LoadBalancerError::HttpClient { .. } => StatusCode::BAD_GATEWAY,
            LoadBalancerError::InvalidStrategy { .. } => StatusCode::BAD_REQUEST,
            LoadBalancerError::NoWorkersAvailable => StatusCode::SERVICE_UNAVAILABLE,
            LoadBalancerError::RequestValidation { .. } => StatusCode::BAD_REQUEST,
            LoadBalancerError::JsonParsing { .. } => StatusCode::BAD_REQUEST,
            LoadBalancerError::HttpBodyProcessing { .. } => StatusCode::BAD_REQUEST,
            LoadBalancerError::Utf8Conversion { .. } => StatusCode::BAD_REQUEST,
            LoadBalancerError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            LoadBalancerError::NotFound { .. } => StatusCode::NOT_FOUND,
            LoadBalancerError::Configuration { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> &str {
        match self {
            LoadBalancerError::WorkerConnection { .. } => "Service temporarily unavailable",
            LoadBalancerError::HttpClient { .. } => "Service temporarily unavailable",
            LoadBalancerError::InvalidStrategy { .. } => "Invalid strategy specified",
            LoadBalancerError::NoWorkersAvailable => "No backend services available",
            LoadBalancerError::RequestValidation { .. } => "Invalid request format",
            LoadBalancerError::JsonParsing { .. } => "Invalid JSON format",
            LoadBalancerError::HttpBodyProcessing { .. } => "Request processing failed",
            LoadBalancerError::Utf8Conversion { .. } => "Invalid request encoding",
            LoadBalancerError::Internal { .. } => "Internal server error",
            LoadBalancerError::NotFound { .. } => "Resource not found",
            LoadBalancerError::Configuration { .. } => "Server configuration error",
        }
    }
}

/// From trait implementations for automatic error conversions
impl From<hyper::Error> for LoadBalancerError {
    fn from(err: hyper::Error) -> Self {
        LoadBalancerError::http_body_processing("hyper operation", err)
    }
}

impl From<serde_json::Error> for LoadBalancerError {
    fn from(err: serde_json::Error) -> Self {
        LoadBalancerError::json_parsing("json operation", err)
    }
}

impl From<std::string::FromUtf8Error> for LoadBalancerError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        LoadBalancerError::utf8_conversion("utf8 conversion", err)
    }
}

/// Response helper for direct error to HTTP response conversion
impl LoadBalancerError {
    /// Convert error directly into HTTP response
    pub fn into_response(self) -> hyper::Response<ResponseBody> {
        create_error_response(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_strategy_error() {
        let error = LoadBalancerError::invalid_strategy("invalid_algo");
        
        assert!(matches!(error, LoadBalancerError::InvalidStrategy { .. }));
        assert_eq!(error.status_code(), hyper::StatusCode::BAD_REQUEST);
        assert_eq!(error.user_message(), "Invalid strategy specified");
    }

    #[test]
    fn test_no_workers_available_error() {
        let error = LoadBalancerError::NoWorkersAvailable;
        
        assert_eq!(error.status_code(), hyper::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.user_message(), "No backend services available");
    }
}