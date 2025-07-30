use crate::error::JanusError;
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
    pub fn validate_socket_path(path: &str) -> Result<(), JanusError> {
        // Must be absolute path
        if !Path::new(path).is_absolute() {
            return Err(JanusError::InvalidSocketPath(
                "Socket path must be absolute".to_string()
            ));
        }
        
        // Check for path traversal sequences
        if path.contains("../") || path.contains("..\\") {
            return Err(JanusError::SecurityViolation(
                "Path traversal detected in socket path".to_string()
            ));
        }
        
        // Check for null bytes
        if path.contains('\0') {
            return Err(JanusError::SecurityViolation(
                "Null byte detected in socket path".to_string()
            ));
        }
        
        // Restrict to safe directories (matching Swift exactly)
        let allowed_prefixes = ["/tmp/", "/var/run/", "/var/tmp/"];
        if !allowed_prefixes.iter().any(|prefix| path.starts_with(prefix)) {
            return Err(JanusError::SecurityViolation(
                "Socket path must be in allowed directory (/tmp, /var/run, /var/tmp)".to_string()
            ));
        }
        
        // Check Unix domain socket path length limit (108 characters)
        if path.len() > 108 {
            return Err(JanusError::InvalidSocketPath(
                "Socket path too long (108 character limit)".to_string()
            ));
        }
        
        // Check for invalid characters (beyond standard path chars)
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9._/\-]+$")?;
        if !valid_chars_regex.is_match(path) {
            return Err(JanusError::SecurityViolation(
                "Socket path contains invalid characters".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate channel ID (exact Swift implementation)
    pub fn validate_channel_id(channel_id: &str, config: &JanusClientConfig) -> Result<(), JanusError> {
        if channel_id.is_empty() {
            return Err(JanusError::InvalidChannel(
                "Channel ID cannot be empty".to_string()
            ));
        }
        
        if channel_id.len() > config.max_channel_name_length {
            return Err(JanusError::InvalidChannel(
                format!("Channel ID too long (max {} characters)", config.max_channel_name_length)
            ));
        }
        
        // Character set validation (alphanumeric, hyphens, underscores only)
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9_\-]+$")?;
        if !valid_chars_regex.is_match(channel_id) {
            return Err(JanusError::InvalidChannel(
                "Channel ID can only contain alphanumeric characters, hyphens, and underscores".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate command name (exact Swift implementation)
    pub fn validate_command_name(command_name: &str, config: &JanusClientConfig) -> Result<(), JanusError> {
        if command_name.is_empty() {
            return Err(JanusError::UnknownCommand(
                "Command name cannot be empty".to_string()
            ));
        }
        
        if command_name.len() > config.max_command_name_length {
            return Err(JanusError::UnknownCommand(
                format!("Command name too long (max {} characters)", config.max_command_name_length)
            ));
        }
        
        // Character set validation (alphanumeric, hyphens, underscores only)
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9_\-]+$")?;
        if !valid_chars_regex.is_match(command_name) {
            return Err(JanusError::UnknownCommand(
                "Command name can only contain alphanumeric characters, hyphens, and underscores".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Validate message size (exact Swift implementation)
    pub fn validate_message_size(size: usize, config: &JanusClientConfig) -> Result<(), JanusError> {
        if size > config.max_message_size {
            return Err(JanusError::MessageTooLarge(size, config.max_message_size));
        }
        Ok(())
    }
    
    /// Validate arguments data size (exact Swift implementation)
    pub fn validate_args_size(args: &Option<std::collections::HashMap<String, serde_json::Value>>, config: &JanusClientConfig) -> Result<(), JanusError> {
        if let Some(args_map) = args {
            let args_json = serde_json::to_string(args_map)?;
            let args_size = args_json.len();
            
            if args_size > config.max_args_data_size {
                return Err(JanusError::ResourceLimit(
                    format!("Arguments data too large: {} bytes (limit: {} bytes)", 
                           args_size, config.max_args_data_size)
                ));
            }
        }
        Ok(())
    }
    
    /// Validate UTF-8 text data (exact Swift implementation)
    pub fn validate_utf8_data(data: &[u8]) -> Result<(), JanusError> {
        std::str::from_utf8(data)
            .map_err(|_| JanusError::MalformedData(
                "Invalid UTF-8 data detected".to_string()
            ))?;
        Ok(())
    }
    
    /// Validate JSON structure (objects only, not arrays/primitives)
    pub fn validate_json_structure(value: &serde_json::Value) -> Result<(), JanusError> {
        match value {
            serde_json::Value::Object(_) => Ok(()),
            _ => Err(JanusError::MalformedData(
                "JSON data must be an object, not array or primitive".to_string()
            ))
        }
    }
    
    /// Comprehensive input sanitization
    pub fn sanitize_string_input(input: &str) -> Result<String, JanusError> {
        // Check for null bytes
        if input.contains('\0') {
            return Err(JanusError::SecurityViolation(
                "Null byte detected in string input".to_string()
            ));
        }
        
        // Check for control characters (except tab, newline, carriage return)
        for ch in input.chars() {
            if ch.is_control() && ch != '\t' && ch != '\n' && ch != '\r' {
                return Err(JanusError::SecurityViolation(
                    "Control character detected in string input".to_string()
                ));
            }
        }
        
        // Return sanitized string (for now, just validate and return original)
        Ok(input.to_string())
    }
}