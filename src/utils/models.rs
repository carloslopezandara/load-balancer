/// Common data models used across the application
/// 
/// These structs define all JSON request/response formats used throughout
/// the load balancer system (admin API, workers, etc.)

use serde::{Deserialize, Serialize};

// ================================
// Worker-related models
// ================================

/// Worker health status response
/// 
/// JSON format:
/// ```json
/// {"status": "healthy", "port": 3000}
/// ```
#[derive(Serialize)]
pub struct WorkerHealthResponse {
    pub status: String,
    pub port: u16,
}

/// Worker processing response
/// 
/// JSON format:
/// ```json
/// {"message": "worker on port 3000 is processing GET /work", "port": 3000}
/// ```
#[derive(Serialize)]
pub struct WorkerResponse {
    pub message: String,
    pub port: u16,
}

// ================================
// Admin API models
// ================================

/// Request body for changing load balancing strategy
/// 
/// Expected JSON format: 
/// ```json
/// {"strategy": "round_robin"}
/// ```
/// or
/// ```json  
/// {"strategy": "least_connections"}
/// ```
#[derive(Deserialize, Debug)]
pub struct ChangeStrategyRequest {
    pub strategy: String,
}

/// Response for getting current strategy
/// 
/// JSON format:
/// ```json
/// {"current_strategy": "round_robin"}
/// ```
#[derive(Serialize, Debug)]
pub struct StrategyResponse {
    pub current_strategy: String,
}

/// Response for changing strategy
/// 
/// JSON format:
/// ```json
/// {
///   "message": "Load balancing strategy updated successfully",
///   "strategy": "least_connections"
/// }
/// ```
#[derive(Serialize, Debug)]
pub struct ChangeStrategyResponse {
    /// Success message
    pub message: String,
    /// Name of the activated strategy  
    pub strategy: String,
}

// ================================
// Common error handling
// ================================

/// Standard error response for API endpoints
/// 
/// Used for all error responses across the application
/// 
/// JSON format:
/// ```json
/// {
///   "error": "Invalid JSON format",
///   "details": "JSON parse error: missing field `strategy` at line 1 column 15"
/// }
/// ```
#[derive(Serialize, Debug)]
pub struct ErrorResponse {
    /// Brief error description
    pub error: String,
    /// Optional detailed error information (omitted if None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}