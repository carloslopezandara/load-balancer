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
    /// With alpha=0.1 (half-life ~6.6 samples), 50 samples ensures proper EMA convergence
    pub const MIN_SAMPLES: u64 = 50;

    /// Default cooldown period between strategy switches in seconds
    pub const COOLDOWN_SECONDS: u64 = 60;

    /// Default evaluation interval for adaptive mode in seconds
    pub const EVALUATION_INTERVAL_SECONDS: u64 = 5;

    /// Default EMA alpha value (smoothing factor)
    /// 0.1 provides balanced responsiveness with half-life of ~6.6 samples
    pub const EMA_ALPHA: f64 = 0.1;
}

/// Metrics calculation constants
pub mod metrics {
    /// Fixed-point scaling factor for storing fractional values as integers
    /// Used for both alpha coefficient and error rate storage in EMA calculations
    pub const FIXED_POINT_SCALE: u32 = 10000;
}
