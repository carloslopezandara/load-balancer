/// Request context for structured logging

use uuid::Uuid;

/// Context for an HTTP request, used for tracking and logging
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub method: String,
    pub path: String,
}

impl RequestContext {
    pub fn new(method: String, path: String) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext::new("GET".to_string(), "/api/test".to_string());
        
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/test");
        assert!(!ctx.request_id.is_empty());
    }

    #[test]
    fn test_request_id_uniqueness() {
        let ctx1 = RequestContext::new("GET".to_string(), "/".to_string());
        let ctx2 = RequestContext::new("GET".to_string(), "/".to_string());
        
        assert_ne!(ctx1.request_id, ctx2.request_id);
    }

    #[test]
    fn test_request_id_format() {
        let ctx = RequestContext::new("POST".to_string(), "/api/data".to_string());
        
        // UUID v4 format: 8-4-4-4-12 characters separated by hyphens
        let parts: Vec<&str> = ctx.request_id.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }
}
