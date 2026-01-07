/// Configuration management using Clap CLI

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use validator::Validate;

use crate::domain::{Result, LoadBalancerError};
use crate::utils::constants::adaptive_defaults;

/// Load Balancer CLI Configuration
#[derive(Parser, Debug)]
#[command(name = "load_balancer")]
#[command(about = "A high-performance HTTP load balancer")]
#[command(version)]
pub struct Config {
    /// Port to listen on
    #[arg(short, long, default_value = "1337")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Worker hosts (can specify multiple)
    #[arg(short, long = "worker", action = clap::ArgAction::Append)]
    pub workers: Vec<String>,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Load balancing strategy
    #[arg(short, long, default_value = "least_connections")]
    pub strategy: String,

    /// Log level
    #[arg(long, default_value = "info")]
    pub log_level: String,
}

/// Configuration that can be loaded from file
#[derive(Serialize, Deserialize, Debug, Validate)]
pub struct LoadBalancerConfig {
    /// Server configuration
    pub server: ServerConfig,
    
    /// Worker configuration
    pub workers: WorkersConfig,
    
    /// Adaptive load balancing configuration
    #[serde(default)]
    #[validate]
    pub adaptive: AdaptiveConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Server configuration
#[derive(Serialize, Deserialize, Debug, Validate)]
pub struct ServerConfig {
    #[validate(range(min = 1024, max = 65535))]
    pub port: u16,
    
    #[validate(length(min = 1))]
    pub host: String,
    
    #[validate(length(min = 1))]
    pub strategy: String,
    
    /// Graceful shutdown timeout in seconds
    #[serde(default = "default_shutdown_timeout")]
    #[validate(range(min = 1, message = "shutdown_timeout_seconds must be at least 1"))]
    pub shutdown_timeout_seconds: u64,
    
    /// Enable adaptive load balancing
    #[serde(default)]
    pub adaptive: bool,
}

/// Workers configuration with validation
#[derive(Serialize, Deserialize, Debug, Validate)]
pub struct WorkersConfig {
    #[validate(length(min = 1, message = "At least one worker must be configured"))]
    pub hosts: Vec<WorkerHost>,
}

/// Individual worker host configuration
#[derive(Serialize, Deserialize, Debug, Validate)]
pub struct WorkerHost {
    #[validate(url(message = "Worker host must be a valid URL"))]
    pub url: String,
    
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Artificial delay in milliseconds for testing (0 = no delay)
    #[serde(default)]
    pub artificial_delay_ms: u64,
    
    /// Artificial error rate for testing (0.0-1.0, where 0.0 = no errors, 1.0 = 100% errors)
    #[serde(default)]
    #[validate(range(min = 0.0, max = 1.0, message = "artificial_error_rate must be between 0.0 and 1.0"))]
    pub artificial_error_rate: f64,
}

/// Adaptive load balancing configuration
#[derive(Serialize, Deserialize, Debug, Validate)]
pub struct AdaptiveConfig {
    /// Maximum acceptable average response time in milliseconds (100-5000ms)
    #[validate(range(min = 100, max = 5000, message = "high_latency_ms must be between 100 and 5000"))]
    pub high_latency_ms: u64,
    
    /// Maximum acceptable error rate (0.01-1.0, where 0.1 = 10%, 1.0 = 100%)
    #[validate(range(min = 0.01, max = 1.0, message = "high_error_rate must be between 0.01 and 1.0"))]
    pub high_error_rate: f64,
    
    /// Minimum number of requests before making decisions
    #[validate(range(min = 1, max = 1000, message = "min_samples must be between 1 and 1000"))]
    pub min_samples: u64,
    
    /// Cooldown period between strategy switches in seconds
    #[validate(range(min = 10, max = 600, message = "cooldown_seconds must be between 10 and 600"))]
    pub cooldown_seconds: u64,
    
    /// Evaluation interval for adaptive mode in seconds
    #[validate(range(min = 1, message = "evaluation_interval_seconds must be at least 1"))]
    pub evaluation_interval_seconds: u64,
    
    /// EMA alpha coefficient for metrics tracking (0.01-1.0)
    /// Controls responsiveness: higher = more reactive, lower = smoother
    /// Default: 0.1 (10% weight to new samples)
    #[serde(default = "default_ema_alpha")]
    #[validate(range(min = 0.01, max = 1.0, message = "ema_alpha must be between 0.01 and 1.0"))]
    pub ema_alpha: f64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            high_latency_ms: adaptive_defaults::HIGH_LATENCY_MS,
            high_error_rate: adaptive_defaults::HIGH_ERROR_RATE,
            min_samples: adaptive_defaults::MIN_SAMPLES,
            cooldown_seconds: adaptive_defaults::COOLDOWN_SECONDS,
            evaluation_interval_seconds: adaptive_defaults::EVALUATION_INTERVAL_SECONDS,
            ema_alpha: adaptive_defaults::EMA_ALPHA,
        }
    }
}

fn default_shutdown_timeout() -> u64 {
    30
}

fn default_ema_alpha() -> f64 {
    adaptive_defaults::EMA_ALPHA
}

/// Logging configuration
#[derive(Serialize, Deserialize, Debug)]
pub struct LoggingConfig {
    pub level: String,
}

impl Config {
    /// Parse configuration from CLI args and optional config file
    pub fn parse_config() -> Result<LoadBalancerConfig> {
        let cli = Config::parse();
        
        if let Some(config_path) = cli.config {
            Self::load_from_file(config_path)
        } else {
            Self::from_cli(cli)
        }
    }
    
    /// Load configuration from file
    fn load_from_file(path: PathBuf) -> Result<LoadBalancerConfig> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| LoadBalancerError::configuration(
                format!("Failed to read config file {:?}: {}", path, e)
            ))?;
            
        let config: LoadBalancerConfig = toml::from_str(&content)
            .map_err(|e| LoadBalancerError::configuration(
                format!("Invalid config file format: {}", e)
            ))?;
            
        config.validate()
            .map_err(|e| LoadBalancerError::configuration(
                format!("Configuration validation failed: {}", e)
            ))?;
            
        Ok(config)
    }
    
    /// Create configuration from CLI arguments
    fn from_cli(cli: Config) -> Result<LoadBalancerConfig> {
        let worker_hosts = if cli.workers.is_empty() {
            vec![
                WorkerHost {
                    url: "http://localhost:3000".to_string(),
                    enabled: true,
                    artificial_delay_ms: 0,
                    artificial_error_rate: 0.0,
                },
                WorkerHost {
                    url: "http://localhost:3001".to_string(),
                    enabled: true,
                    artificial_delay_ms: 0,
                    artificial_error_rate: 0.0,
                }
            ]
        } else {
            cli.workers.into_iter().map(|url| WorkerHost {
                url,
                enabled: true,
                artificial_delay_ms: 0,
                artificial_error_rate: 0.0,
            }).collect()
        };
        
        let config = LoadBalancerConfig {
            server: ServerConfig {
                port: cli.port,
                host: cli.host,
                strategy: cli.strategy,
                adaptive: false,
                shutdown_timeout_seconds: default_shutdown_timeout(),
            },
            workers: WorkersConfig {
                hosts: worker_hosts,
            },
            adaptive: AdaptiveConfig::default(),
            logging: LoggingConfig {
                level: cli.log_level,
            },
        };
        
        config.validate()
            .map_err(|e| LoadBalancerError::configuration(
                format!("Configuration validation failed: {}", e)
            ))?;
            
        Ok(config)
    }
}

fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = LoadBalancerConfig {
            server: ServerConfig {
                port: 8080,
                host: "0.0.0.0".to_string(),
                strategy: "round_robin".to_string(),
                adaptive: false,
                shutdown_timeout_seconds: 30,
            },
            workers: WorkersConfig {
                hosts: vec![
                    WorkerHost {
                        url: "http://localhost:3000".to_string(),
                        enabled: true,
                        artificial_delay_ms: 0,
                        artificial_error_rate: 0.0,
                    }
                ],
            },
            adaptive: AdaptiveConfig::default(),
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_adaptive_config_defaults() {
        let adaptive = AdaptiveConfig::default();
        
        assert_eq!(adaptive.high_latency_ms, adaptive_defaults::HIGH_LATENCY_MS);
        assert_eq!(adaptive.high_error_rate, adaptive_defaults::HIGH_ERROR_RATE);
        assert_eq!(adaptive.min_samples, adaptive_defaults::MIN_SAMPLES);
        assert_eq!(adaptive.cooldown_seconds, adaptive_defaults::COOLDOWN_SECONDS);
        assert_eq!(adaptive.evaluation_interval_seconds, adaptive_defaults::EVALUATION_INTERVAL_SECONDS);
    }

    #[test]
    fn test_adaptive_config_validation_valid() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 1000,
            high_error_rate: 0.05,
            min_samples: 20,
            cooldown_seconds: 120,
            evaluation_interval_seconds: 10,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_ok());
    }

    #[test]
    fn test_adaptive_config_validation_latency_too_low() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 50, // Below minimum of 100
            high_error_rate: 0.1,
            min_samples: 10,
            cooldown_seconds: 60,
            evaluation_interval_seconds: 5,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_latency_too_high() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 6000, // Above maximum of 5000
            high_error_rate: 0.1,
            min_samples: 10,
            cooldown_seconds: 60,
            evaluation_interval_seconds: 5,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_error_rate_too_low() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 500,
            high_error_rate: 0.005, // Below minimum of 0.01
            min_samples: 10,
            cooldown_seconds: 60,
            evaluation_interval_seconds: 5,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_error_rate_too_high() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 500,
            high_error_rate: 1.5, // Above maximum of 1.0
            min_samples: 10,
            cooldown_seconds: 60,
            evaluation_interval_seconds: 5,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_min_samples_zero() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 500,
            high_error_rate: 0.1,
            min_samples: 0, // Below minimum of 1
            cooldown_seconds: 60,
            evaluation_interval_seconds: 5,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_cooldown_too_low() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 500,
            high_error_rate: 0.1,
            min_samples: 10,
            cooldown_seconds: 5, // Below minimum of 10
            evaluation_interval_seconds: 5,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_cooldown_too_high() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 500,
            high_error_rate: 0.1,
            min_samples: 10,
            cooldown_seconds: 700, // Above maximum of 600
            evaluation_interval_seconds: 5,
            ema_alpha: 0.1,
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_ema_alpha_too_low() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 500,
            high_error_rate: 0.1,
            min_samples: 10,
            cooldown_seconds: 60,
            evaluation_interval_seconds: 5,
            ema_alpha: 0.005, // Below minimum of 0.01
        };
        
        assert!(adaptive.validate().is_err());
    }

    #[test]
    fn test_adaptive_config_validation_ema_alpha_too_high() {
        let adaptive = AdaptiveConfig {
            high_latency_ms: 500,
            high_error_rate: 0.1,
            min_samples: 10,
            cooldown_seconds: 60,
            evaluation_interval_seconds: 5,
            ema_alpha: 1.5, // Above maximum of 1.0
        };
        
        assert!(adaptive.validate().is_err());
    }

}