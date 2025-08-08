use crate::error::{JSONRPCError, JSONRPCErrorCode};
use crate::config::JanusClientConfig;
use std::path::Path;
use regex::Regex;

/// Security validation framework matching SwiftJanus exactly
#[derive(Debug)]
pub struct SecurityValidator;

impl SecurityValidator {
    /// Create a new SecurityValidator instance
    pub fn new() -> Self {
        SecurityValidator
    }
    /// Validate socket path for security (exact Swift implementation)
    pub fn validate_socket_path(path: &str) -> Result<(), JSONRPCError> {
        // Must be absolute path
        if !Path::new(path).is_absolute() {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::ValidationFailed,
                Some("Socket path must be absolute".to_string())
            ));
        }
        
        // Check for path traversal sequences
        if path.contains("../") || path.contains("..\\") {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::SecurityViolation,
                Some("Path traversal detected in socket path".to_string())
            ));
        }
        
        // Check for null bytes
        if path.contains('\0') {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::SecurityViolation,
                Some("Null byte detected in socket path".to_string())
            ));
        }
        
        // Restrict to safe directories (matching Swift exactly)
        let allowed_prefixes = ["/tmp/", "/var/run/", "/var/tmp/"];
        if !allowed_prefixes.iter().any(|prefix| path.starts_with(prefix)) {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::SecurityViolation,
                Some("Socket path must be in allowed directory (/tmp, /var/run, /var/tmp)".to_string())
            ));
        }
        
        // Check Unix domain socket path length limit (108 characters)
        if path.len() > 108 {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::ValidationFailed,
                Some("Socket path too long (108 character limit)".to_string())
            ));
        }
        
        // Check for invalid characters (beyond standard path chars)
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9._/\-]+$")?;
        if !valid_chars_regex.is_match(path) {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::SecurityViolation,
                Some("Socket path contains invalid characters".to_string())
            ));
        }
        
        Ok(())
    }
    
    /// Validate channel ID (exact Swift implementation)
    pub fn validate_channel_id(channel_id: &str, config: &JanusClientConfig) -> Result<(), JSONRPCError> {
        if channel_id.is_empty() {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::InvalidRequest,
                Some("Channel ID cannot be empty".to_string())
            ));
        }
        
        if channel_id.len() > config.max_channel_name_length {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::InvalidRequest,
                Some(format!("Channel ID too long (max {} characters)", config.max_channel_name_length))
            ));
        }
        
        // Character set validation (alphanumeric, hyphens, underscores only)
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9_\-]+$")?;
        if !valid_chars_regex.is_match(channel_id) {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::InvalidRequest,
                Some("Channel ID can only contain alphanumeric characters, hyphens, and underscores".to_string())
            ));
        }
        
        Ok(())
    }
    
    /// Validate request name (exact Swift implementation)
    pub fn validate_request_name(request_name: &str, config: &JanusClientConfig) -> Result<(), JSONRPCError> {
        if request_name.is_empty() {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::MethodNotFound,
                Some("Request name cannot be empty".to_string())
            ));
        }
        
        if request_name.len() > config.max_request_name_length {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::MethodNotFound,
                Some(format!("Request name too long (max {} characters)", config.max_request_name_length))
            ));
        }
        
        // Character set validation (alphanumeric, hyphens, underscores only)
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9_\-]+$")?;
        if !valid_chars_regex.is_match(request_name) {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::MethodNotFound,
                Some("Request name can only contain alphanumeric characters, hyphens, and underscores".to_string())
            ));
        }
        
        Ok(())
    }
    
    /// Validate message size (exact Swift implementation)
    pub fn validate_message_size(size: usize, config: &JanusClientConfig) -> Result<(), JSONRPCError> {
        if size > config.max_message_size {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::ResourceLimitExceeded,
                Some(format!("Message too large: {} bytes (limit: {} bytes)", size, config.max_message_size))
            ));
        }
        Ok(())
    }
    
    /// Validate arguments data size (exact Swift implementation)
    pub fn validate_args_size(args: &Option<std::collections::HashMap<String, serde_json::Value>>, config: &JanusClientConfig) -> Result<(), JSONRPCError> {
        if let Some(args_map) = args {
            let args_json = serde_json::to_string(args_map)?;
            let args_size = args_json.len();
            
            if args_size > config.max_args_data_size {
                return Err(JSONRPCError::new(
                    JSONRPCErrorCode::ResourceLimitExceeded,
                    Some(format!("Arguments data too large: {} bytes (limit: {} bytes)", 
                           args_size, config.max_args_data_size))
                ));
            }
        }
        Ok(())
    }
    
    /// Validate UTF-8 text data (exact Swift implementation)
    pub fn validate_utf8_data(data: &[u8]) -> Result<(), JSONRPCError> {
        std::str::from_utf8(data)
            .map_err(|_| JSONRPCError::new(
                JSONRPCErrorCode::ValidationFailed,
                Some("Invalid UTF-8 data detected".to_string())
            ))?;
        Ok(())
    }
    
    /// Validate JSON structure (objects only, not arrays/primitives)
    pub fn validate_json_structure(value: &serde_json::Value) -> Result<(), JSONRPCError> {
        match value {
            serde_json::Value::Object(_) => Ok(()),
            _ => Err(JSONRPCError::new(
                JSONRPCErrorCode::ValidationFailed,
                Some("JSON data must be an object, not array or primitive".to_string())
            ))
        }
    }
    
    /// Comprehensive input sanitization
    pub fn sanitize_string_input(input: &str) -> Result<String, JSONRPCError> {
        // Check for null bytes
        if input.contains('\0') {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::SecurityViolation,
                Some("Null byte detected in string input".to_string())
            ));
        }
        
        // Check for control characters (except tab, newline, carriage return)
        for ch in input.chars() {
            if ch.is_control() && ch != '\t' && ch != '\n' && ch != '\r' {
                return Err(JSONRPCError::new(
                    JSONRPCErrorCode::SecurityViolation,
                    Some("Control character detected in string input".to_string())
                ));
            }
        }
        
        // Return sanitized string (for now, just validate and return original)
        Ok(input.to_string())
    }
    
    /// Validate UUID format (matches TypeScript implementation)
    pub fn validate_uuid_format(uuid: &str) -> Result<(), JSONRPCError> {
        // UUID v4 format: 8-4-4-4-12 hexadecimal digits
        let uuid_regex = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")?;
        if !uuid_regex.is_match(uuid) {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::SecurityViolation,
                Some(format!("Invalid UUID format: {}", uuid))
            ));
        }
        Ok(())
    }
    
    /// Validate timestamp format (matches Swift/TypeScript implementation)
    pub fn validate_timestamp_format(timestamp: &str) -> Result<(), JSONRPCError> {
        use chrono::{DateTime, Utc};
        
        // Try parsing as ISO 8601 format
        if DateTime::parse_from_rfc3339(timestamp).is_ok() {
            return Ok(());
        }
        
        // Try parsing as UTC format
        if timestamp.parse::<DateTime<Utc>>().is_ok() {
            return Ok(());
        }
        
        Err(JSONRPCError::new(
            JSONRPCErrorCode::SecurityViolation,
            Some(format!("Invalid timestamp format: {} (expected ISO 8601)", timestamp))
        ))
    }

    /// Validate reserved channel names (matches Swift implementation)
    pub fn validate_reserved_channels(&self, channel_id: &str) -> Result<(), JSONRPCError> {
        let reserved_channels = [
            "system", "admin", "root", "internal", "__proto__", "constructor"
        ];
        
        let lower_channel = channel_id.to_lowercase();
        if reserved_channels.contains(&lower_channel.as_str()) {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::SecurityViolation,
                Some(format!("Channel ID '{}' is reserved and cannot be used", channel_id))
            ));
        }
        
        Ok(())
    }

    /// Validate dangerous request patterns (matches Swift implementation)
    pub fn validate_dangerous_request(&self, request_name: &str) -> Result<(), JSONRPCError> {
        let dangerous_patterns = ["eval", "exec", "system", "shell", "rm", "delete", "drop"];
        let lower_request = request_name.to_lowercase();
        
        for pattern in &dangerous_patterns {
            if lower_request.contains(pattern) {
                return Err(JSONRPCError::new(
                    JSONRPCErrorCode::SecurityViolation,
                    Some(format!("Request name contains dangerous pattern: {}", pattern))
                ));
            }
        }
        Ok(())
    }

    /// Validate argument security (matches Swift implementation)
    pub fn validate_argument_security(&self, args: &serde_json::Map<String, serde_json::Value>) -> Result<(), JSONRPCError> {
        let dangerous_args = [
            "__proto__", "constructor", "prototype", "eval", "function"
        ];
        
        for arg_name in args.keys() {
            let lower_arg = arg_name.to_lowercase();
            if dangerous_args.contains(&lower_arg.as_str()) {
                return Err(JSONRPCError::new(
                    JSONRPCErrorCode::SecurityViolation,
                    Some(format!("Dangerous argument name: {}", arg_name))
                ));
            }
        }
        
        // Validate argument values for injection attempts
        for (key, value) in args {
            self.validate_argument_value(key, value)?;
        }
        
        Ok(())
    }

    /// Validate argument value for SQL and script injection patterns (matches Swift implementation)
    fn validate_argument_value(&self, key: &str, value: &serde_json::Value) -> Result<(), JSONRPCError> {
        if let Some(string_value) = value.as_str() {
            let lower_value = string_value.to_lowercase();
            
            // Check for SQL injection patterns
            let sql_patterns = ["'", "\"", "--", "/*", "*/", "union", "select", "drop", "delete", "insert", "update"];
            for pattern in &sql_patterns {
                if lower_value.contains(pattern) {
                    return Err(JSONRPCError::new(
                        JSONRPCErrorCode::SecurityViolation,
                        Some(format!("Argument '{}' contains potentially dangerous SQL pattern: {}", key, pattern))
                    ));
                }
            }
            
            // Check for script injection patterns
            let script_patterns = ["<script", "javascript:", "vbscript:", "onload=", "onerror="];
            for pattern in &script_patterns {
                if lower_value.contains(pattern) {
                    return Err(JSONRPCError::new(
                        JSONRPCErrorCode::SecurityViolation,
                        Some(format!("Argument '{}' contains script injection pattern: {}", key, pattern))
                    ));
                }
            }
        }
        Ok(())
    }
}