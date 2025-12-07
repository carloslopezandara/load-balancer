/// Administrative Endpoint Handlers
/// 
/// This module contains the HTTP handlers for load balancer administration.
/// All handlers follow REST principles and return JSON responses.

use std::sync::Arc;
use hyper::{body::Incoming, Request, Response, Method, StatusCode};
use hyper_util::client::legacy::Error as ClientError;

use crate::ResponseBody;
use crate::load_balancer::LoadBalancer;
use crate::strategy::LoadBalancingStrategy;
use crate::utils::{create_error_response, create_json_response, ChangeStrategyRequest, StrategyResponse, ChangeStrategyResponse};

/// Handles all administrative endpoints for managing the load balancer
/// 
/// This is the main entry point for admin API requests. It routes requests
/// to appropriate handlers based on HTTP method and path.
pub async fn handle_admin(
    req: Request<Incoming>,
    load_balancer: Arc<LoadBalancer>
) -> Result<Response<ResponseBody>, ClientError> {
    let method = req.method();
    let path = req.uri().path();
    
    match (method, path) {
        // GET /admin/strategy - Return current strategy
        (&Method::GET, "/admin/strategy") => {
            let strategy_name = load_balancer.get_strategy_name().await;
            let response = StrategyResponse {
                current_strategy: strategy_name,
            };
            
            Ok(create_json_response(&response))
        }
        
        // POST /admin/strategy - Change strategy
        (&Method::POST, "/admin/strategy") => {
            handle_strategy_change(req, load_balancer).await
        }
        
        // Handle not supported admin endpoints
        _ => {
            Ok(create_error_response(
                StatusCode::NOT_FOUND,
                "Admin endpoint not found",
                Some(&format!(
                    "'{} {}' is not a valid admin endpoint. Available: GET /admin/strategy, POST /admin/strategy",
                    method, path
                ))
            ))
        }
    }
}

/// Handles POST /admin/strategy requests for changing load balancing strategy
/// 
/// This function implements the complete flow for strategy changes
async fn handle_strategy_change(
    req: Request<Incoming>,
    load_balancer: Arc<LoadBalancer>
) -> Result<Response<ResponseBody>, ClientError> {
    // Read and parse request body
    let body_result = http_body_util::BodyExt::collect(req.into_body()).await;
    let body_bytes = match body_result {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return Ok(create_error_response(
                StatusCode::BAD_REQUEST,
                "Failed to read request body",
                Some(&format!("Body read error: {}", e))
            ));
        }
    };
    
    // Convert to string for JSON parsing
    let body_str = match String::from_utf8(body_bytes.to_vec()) {
        Ok(body) => body,
        Err(e) => {
            return Ok(create_error_response(
                StatusCode::BAD_REQUEST,
                "Request body is not valid UTF-8",
                Some(&format!("UTF-8 conversion error: {}", e))
            ));
        }
    };
    
    // Parse JSON using serde
    let strategy_request: ChangeStrategyRequest = match serde_json::from_str(&body_str) {
        Ok(req) => req,
        Err(e) => {
            return Ok(create_error_response(
                StatusCode::BAD_REQUEST,
                "Invalid JSON format",
                Some(&format!("JSON parse error: {}", e))
            ));
        }
    };
    
    // Validate strategy name and check if it's already the current strategy
    let current_strategy_name = load_balancer.get_strategy_name().await;
    
    // Early return if the requested strategy is already active
    if strategy_request.strategy == current_strategy_name {
        let response = ChangeStrategyResponse {
            message: format!("Strategy '{}' is already active", current_strategy_name),
            strategy: current_strategy_name,
        };
        return Ok(create_json_response(&response));
    }
    
    let new_strategy = match strategy_request.strategy.as_str() {
        "round_robin" => LoadBalancingStrategy::new_round_robin(),
        "least_connections" => {
            let worker_count = load_balancer.worker_hosts().len();
            LoadBalancingStrategy::new_least_connections(worker_count)
        },
        invalid_strategy => {
            return Ok(create_error_response(
                StatusCode::BAD_REQUEST,
                "Invalid strategy name",
                Some(&format!(
                    "'{}' is not a valid strategy. Valid strategies are: 'round_robin', 'least_connections'",
                    invalid_strategy
                ))
            ));
        }
    };
    
    // Apply the strategy change
    load_balancer.set_strategy(new_strategy).await;
    
    // Return success response
    let response = ChangeStrategyResponse {
        message: "Load balancing strategy updated successfully".to_string(),
        strategy: strategy_request.strategy,
    };
    
    Ok(create_json_response(&response))
}