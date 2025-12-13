//! Admin service - business logic for administrative operations
//! 
//! This service handles all administrative business logic including
//! strategy management, worker configuration, and status reporting.
//! It's independent of HTTP concerns and can be used by any interface.

use hyper::{Request, Response};
use hyper::body::Incoming;
use std::sync::Arc;
use crate::domain::{
    LoadBalancerError,
    Result,
    StrategyType,
    StrategyResponse,
    ChangeStrategyResponse,
    ChangeStrategyRequest,
};
use crate::utils::create_json_response_with_status;
use crate::utils::http_utils::{create_fallback_error_response, parse_json_body};
use crate::load_balancing_strategy::LoadBalancingStrategy;
use crate::services::load_balancer::LoadBalancerService;
use crate::ResponseBody;

/// Admin service for handling administrative business logic
/// 
/// This service implements all administrative operations including
/// strategy management and worker monitoring.
pub struct AdminService {
    load_balancer_service: Arc<LoadBalancerService>,
}

impl AdminService {
    /// Create new admin service
    pub fn new(load_balancer_service: Arc<LoadBalancerService>) -> Self {
        Self { load_balancer_service }
    }

    /// Get the current load balancing strategy
    /// 
    /// Returns the currently active strategy with proper typing.
    pub async fn get_current_strategy(&self) -> Result<Response<ResponseBody>> {
        tracing::debug!("Getting current load balancing strategy");
        
        let strategy_name = self.load_balancer_service.get_strategy_name().await;
        
        let strategy = StrategyType::from_str(strategy_name).map_err(|_e| {
            LoadBalancerError::internal(format!("Invalid strategy stored: {}", strategy_name))
        })?;
        
        let response = StrategyResponse::new(&strategy);
        
        tracing::info!("Current strategy: {}", strategy_name);
        Ok(create_json_response_with_status(&response, hyper::StatusCode::OK)
            .unwrap_or_else(|e| {
                tracing::error!("Failed to serialize admin response: {}", e);
                create_fallback_error_response()
            }))
    }

    /// Change the load balancing strategy
    /// 
    /// Accepts a strategy change request and updates the load balancer
    /// configuration with proper validation.
    pub async fn change_strategy(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<ResponseBody>> {
        tracing::debug!("Processing strategy change request");
        
        // Parse and validate JSON request body
        let change_request: ChangeStrategyRequest = parse_json_body(req).await?;
        
        let strategy_str = change_request.strategy.to_string();
        
        tracing::info!(
            "Attempting to change strategy to: {}", 
            strategy_str
        );
        
        let current_strategy_name = self.load_balancer_service.get_strategy_name().await;
        
        if current_strategy_name == strategy_str {
            tracing::info!("Strategy already set to: {}", strategy_str);
            let response = ChangeStrategyResponse::new(&change_request.strategy);
            return Ok(create_json_response_with_status(&response, hyper::StatusCode::OK)
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to serialize admin response: {}", e);
                    create_fallback_error_response()
                }));
        }
        
        let new_strategy = match change_request.strategy {
            StrategyType::RoundRobin => LoadBalancingStrategy::new_round_robin(),
            StrategyType::LeastConnections => {
                let worker_count = self.load_balancer_service.worker_hosts().len();
                LoadBalancingStrategy::new_least_connections(worker_count)?
            },
        };
        
        self.load_balancer_service.set_strategy(new_strategy).await;
        
        // Verify the change was successful
        let actual_strategy_name = self.load_balancer_service.get_strategy_name().await;
        if actual_strategy_name != strategy_str {
            return Err(LoadBalancerError::internal(
                "Strategy change verification failed".to_string()
            ));
        }
        
        tracing::info!(
            "Successfully changed strategy from {} to {}", 
            current_strategy_name, 
            actual_strategy_name
        );
        
        let response = ChangeStrategyResponse::new(&change_request.strategy);
        Ok(create_json_response_with_status(&response, hyper::StatusCode::OK)
            .unwrap_or_else(|e| {
                tracing::error!("Failed to serialize admin response: {}", e);
                create_fallback_error_response()
            }))
    }

}
