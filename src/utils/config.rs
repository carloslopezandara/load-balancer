/// Configuration management using Clap CLI

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use validator::Validate;

use crate::domain::{Result, LoadBalancerError};

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
                },
                WorkerHost {
                    url: "http://localhost:3001".to_string(),
                    enabled: true,
                }
            ]
        } else {
            cli.workers.into_iter().map(|url| WorkerHost {
                url,
                enabled: true,
            }).collect()
        };
        
        let config = LoadBalancerConfig {
            server: ServerConfig {
                port: cli.port,
                host: cli.host,
                strategy: cli.strategy,
            },
            workers: WorkersConfig {
                hosts: worker_hosts,
            },
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
            },
            workers: WorkersConfig {
                hosts: vec![
                    WorkerHost {
                        url: "http://localhost:3000".to_string(),
                        enabled: true,
                    }
                ],
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        };
        
        assert!(config.validate().is_ok());
    }

}