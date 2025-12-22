/// HTTP Constants
/// 
/// Centralized HTTP-related constants for consistent use across the application.

/// Common HTTP header values
pub mod headers {
    /// Content-Type: application/json
    pub const APPLICATION_JSON: &str = "application/json";
    
    /// Content-Type: text/plain
    pub const TEXT_PLAIN: &str = "text/plain";
}

/// Configuration default values for adaptive load balancing
pub mod adaptive_defaults {
    /// Default threshold for high latency detection in milliseconds
    pub const HIGH_LATENCY_MS: u64 = 500;

    /// Default threshold for high error rate detection (0.1 = 10%)
    pub const HIGH_ERROR_RATE: f64 = 0.1;

    /// Default minimum number of samples required before making decisions
    pub const MIN_SAMPLES: u64 = 10;

    /// Default cooldown period between strategy switches in seconds
    pub const COOLDOWN_SECONDS: u64 = 60;

    /// Default evaluation interval for adaptive mode in seconds
    pub const EVALUATION_INTERVAL_SECONDS: u64 = 5;
}
