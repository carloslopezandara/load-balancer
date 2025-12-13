/// Worker Domain Module
/// 
/// Contains all worker-related types:
/// - WorkerUrl: Validated URL newtype
/// - WorkerResponse: Generic worker response DTO

use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// WorkerUrl - Validated URL Type
// ============================================================================

/// Validated worker URL type
/// 
/// This newtype ensures that all worker URLs are validated at construction time,
/// preventing invalid URLs from propagating through the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkerUrl(String);

impl WorkerUrl {
    /// Parse and validate a worker URL
    /// 
    /// # Arguments
    /// * `s` - The URL string to validate
    /// 
    /// # Returns
    /// * `Ok(WorkerUrl)` if the URL is valid
    /// * `Err` if the URL is invalid
    /// 
    /// # Examples
    /// ```
    /// use load_balancer::domain::WorkerUrl;
    /// 
    /// let url = WorkerUrl::parse("http://localhost:8080".to_string()).unwrap();
    /// ```
    pub fn parse(s: String) -> Result<Self> {
        if validate_worker_url(&s) {
            Ok(Self(s))
        } else {
            Err(eyre!("Invalid worker URL: {}", s))
        }
    }

    /// Get the inner URL string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume self and return the inner string
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Validate a worker URL
/// 
/// Checks that the URL:
/// - Starts with http:// or https://
/// - Contains a valid host (not empty after scheme)
/// - Is not just the scheme
fn validate_worker_url(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return false;
    }

    let host_part = if url.starts_with("https://") {
        &url[8..]
    } else {
        &url[7..]
    };

    if host_part.is_empty() {
        return false;
    }

    !host_part.trim().is_empty()
}

impl AsRef<str> for WorkerUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkerUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for WorkerUrl {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for WorkerUrl {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        WorkerUrl::parse(s).map_err(serde::de::Error::custom)
    }
}

// ============================================================================
// Worker Response DTOs
// ============================================================================

/// Worker generic response model (DTO)
#[derive(Serialize, Debug, Clone)]
pub struct WorkerResponse {
    pub message: String,
    pub port: u16,
}

impl WorkerResponse {
    pub fn new(message: impl Into<String>, port: u16) -> Self {
        Self {
            message: message.into(),
            port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // WorkerUrl Tests
    // ========================================================================

    #[test]
    fn valid_http_url_is_accepted() {
        let url = "http://localhost:8080";
        assert!(WorkerUrl::parse(url.to_string()).is_ok());
    }

    #[test]
    fn valid_https_url_is_accepted() {
        let url = "https://example.com:3000";
        assert!(WorkerUrl::parse(url.to_string()).is_ok());
    }

    #[test]
    fn url_with_path_is_accepted() {
        let url = "http://localhost:8080/api/health";
        assert!(WorkerUrl::parse(url.to_string()).is_ok());
    }

    #[test]
    fn empty_string_is_rejected() {
        let url = "";
        assert!(WorkerUrl::parse(url.to_string()).is_err());
    }

    #[test]
    fn url_without_scheme_is_rejected() {
        let url = "localhost:8080";
        assert!(WorkerUrl::parse(url.to_string()).is_err());
    }

    #[test]
    fn url_with_invalid_scheme_is_rejected() {
        let url = "ftp://localhost:8080";
        assert!(WorkerUrl::parse(url.to_string()).is_err());
    }

    #[test]
    fn url_with_only_scheme_is_rejected() {
        let url = "http://";
        assert!(WorkerUrl::parse(url.to_string()).is_err());
    }

    #[test]
    fn url_with_empty_host_is_rejected() {
        let url = "http://   ";
        assert!(WorkerUrl::parse(url.to_string()).is_err());
    }

    #[test]
    fn as_str_returns_inner_value() {
        let url = WorkerUrl::parse("http://localhost:8080".to_string()).unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080");
    }

    #[test]
    fn into_inner_consumes_and_returns_string() {
        let url = WorkerUrl::parse("http://localhost:8080".to_string()).unwrap();
        let inner = url.into_inner();
        assert_eq!(inner, "http://localhost:8080");
    }

    #[test]
    fn serialization_works() {
        let url = WorkerUrl::parse("http://localhost:8080".to_string()).unwrap();
        let json = serde_json::to_string(&url).unwrap();
        assert_eq!(json, r#""http://localhost:8080""#);
    }

    #[test]
    fn deserialization_works() {
        let json = r#""http://localhost:8080""#;
        let url: WorkerUrl = serde_json::from_str(json).unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080");
    }

    #[test]
    fn deserialization_rejects_invalid_url() {
        let json = r#""invalid-url""#;
        let result: Result<WorkerUrl, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}