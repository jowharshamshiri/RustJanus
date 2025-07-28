use crate::specification::ApiSpecification;
use crate::error::UnixSockApiError;
use tokio::fs;

/// API specification parser for JSON and YAML formats (exact SwiftUnixSockAPI parity)
pub struct ApiSpecificationParser;

impl ApiSpecificationParser {
    /// Parse API specification from JSON string
    pub fn from_json(json_str: &str) -> Result<ApiSpecification, UnixSockApiError> {
        serde_json::from_str(json_str)
            .map_err(|e| UnixSockApiError::DecodingFailed(format!("JSON parsing error: {}", e)))
    }
    
    /// Parse API specification from YAML string
    #[cfg(feature = "yaml-support")]
    pub fn from_yaml(yaml_str: &str) -> Result<ApiSpecification, UnixSockApiError> {
        serde_yaml::from_str(yaml_str)
            .map_err(|e| UnixSockApiError::DecodingFailed(format!("YAML parsing error: {}", e)))
    }
    
    /// Parse API specification from file (auto-detect format based on extension)
    pub async fn from_file(path: &str) -> Result<ApiSpecification, UnixSockApiError> {
        let content = fs::read_to_string(path).await
            .map_err(|e| UnixSockApiError::IoError(format!("Failed to read file {}: {}", path, e)))?;
        
        if path.ends_with(".yaml") || path.ends_with(".yml") {
            #[cfg(feature = "yaml-support")]
            {
                Self::from_yaml(&content)
            }
            #[cfg(not(feature = "yaml-support"))]
            {
                Err(UnixSockApiError::DecodingFailed(
                    "YAML support not enabled. Enable 'yaml-support' feature.".to_string()
                ))
            }
        } else if path.ends_with(".json") {
            Self::from_json(&content)
        } else {
            // Default to JSON if extension is ambiguous
            Self::from_json(&content)
        }
    }
    
    /// Serialize API specification to JSON string
    pub fn to_json(api_spec: &ApiSpecification) -> Result<String, UnixSockApiError> {
        serde_json::to_string_pretty(api_spec)
            .map_err(|e| UnixSockApiError::EncodingFailed(format!("JSON serialization error: {}", e)))
    }
    
    /// Serialize API specification to YAML string
    #[cfg(feature = "yaml-support")]
    pub fn to_yaml(api_spec: &ApiSpecification) -> Result<String, UnixSockApiError> {
        serde_yaml::to_string(api_spec)
            .map_err(|e| UnixSockApiError::EncodingFailed(format!("YAML serialization error: {}", e)))
    }
    
    /// Write API specification to file (format based on extension)
    pub async fn to_file(api_spec: &ApiSpecification, path: &str) -> Result<(), UnixSockApiError> {
        let content = if path.ends_with(".yaml") || path.ends_with(".yml") {
            #[cfg(feature = "yaml-support")]
            {
                Self::to_yaml(api_spec)?
            }
            #[cfg(not(feature = "yaml-support"))]
            {
                return Err(UnixSockApiError::EncodingFailed(
                    "YAML support not enabled. Enable 'yaml-support' feature.".to_string()
                ));
            }
        } else {
            Self::to_json(api_spec)?
        };
        
        fs::write(path, content).await
            .map_err(|e| UnixSockApiError::IoError(format!("Failed to write file {}: {}", path, e)))?;
        
        Ok(())
    }
    
    /// Validate API specification structure and content
    pub fn validate(api_spec: &ApiSpecification) -> Result<(), UnixSockApiError> {
        // Validate version
        if api_spec.version.is_empty() {
            return Err(UnixSockApiError::MalformedData(
                "API specification version is required".to_string()
            ));
        }
        
        // Validate version format (semantic versioning)
        if !Self::is_valid_version(&api_spec.version) {
            return Err(UnixSockApiError::MalformedData(
                format!("Invalid version format: {}", api_spec.version)
            ));
        }
        
        // Validate channels
        if api_spec.channels.is_empty() {
            return Err(UnixSockApiError::MalformedData(
                "API specification must define at least one channel".to_string()
            ));
        }
        
        for (channel_name, channel_spec) in &api_spec.channels {
            Self::validate_channel(channel_name, channel_spec)?;
        }
        
        // Validate models if present
        if let Some(models) = &api_spec.models {
            for (model_name, model_spec) in models {
                Self::validate_model(model_name, model_spec, api_spec)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate semantic version format
    fn is_valid_version(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        
        parts.iter().all(|part| part.parse::<u32>().is_ok())
    }
    
    /// Validate channel specification
    fn validate_channel(
        channel_name: &str,
        channel_spec: &crate::specification::ChannelSpec,
    ) -> Result<(), UnixSockApiError> {
        // Channel name validation
        if channel_name.is_empty() {
            return Err(UnixSockApiError::InvalidChannel(
                "Channel name cannot be empty".to_string()
            ));
        }
        
        // Channel name format validation (alphanumeric, hyphens, underscores)
        if !channel_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(UnixSockApiError::InvalidChannel(
                format!("Invalid channel name format: {}", channel_name)
            ));
        }
        
        // Description validation
        if channel_spec.description.is_empty() {
            return Err(UnixSockApiError::InvalidChannel(
                format!("Channel '{}' must have a description", channel_name)
            ));
        }
        
        // Commands validation
        if channel_spec.commands.is_empty() {
            return Err(UnixSockApiError::InvalidChannel(
                format!("Channel '{}' must define at least one command", channel_name)
            ));
        }
        
        for (command_name, command_spec) in &channel_spec.commands {
            Self::validate_command_spec(channel_name, command_name, command_spec)?;
        }
        
        Ok(())
    }
    
    /// Validate command specification
    fn validate_command_spec(
        channel_name: &str,
        command_name: &str,
        command_spec: &crate::specification::CommandSpec,
    ) -> Result<(), UnixSockApiError> {
        // Command name validation
        if command_name.is_empty() {
            return Err(UnixSockApiError::UnknownCommand(
                "Command name cannot be empty".to_string()
            ));
        }
        
        // Command name format validation
        if !command_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(UnixSockApiError::UnknownCommand(
                format!("Invalid command name format: {}", command_name)
            ));
        }
        
        // Description validation
        if command_spec.description.is_empty() {
            return Err(UnixSockApiError::UnknownCommand(
                format!("Command '{}' in channel '{}' must have a description", 
                       command_name, channel_name)
            ));
        }
        
        // Validate arguments
        for (arg_name, arg_spec) in &command_spec.args {
            Self::validate_argument_spec(arg_name, arg_spec)?;
        }
        
        // Validate response spec
        Self::validate_response_spec(&command_spec.response)?;
        
        // Validate error codes if present
        if let Some(error_codes) = &command_spec.error_codes {
            for (error_name, error_spec) in error_codes {
                Self::validate_error_code_spec(error_name, error_spec)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate argument specification
    fn validate_argument_spec(
        arg_name: &str,
        arg_spec: &crate::specification::ArgumentSpec,
    ) -> Result<(), UnixSockApiError> {
        // Argument name validation
        if arg_name.is_empty() {
            return Err(UnixSockApiError::InvalidArgument(
                "argument".to_string(),
                "Argument name cannot be empty".to_string()
            ));
        }
        
        // Type validation
        let valid_types = ["string", "integer", "number", "boolean", "array", "object"];
        if !valid_types.contains(&arg_spec.r#type.as_str()) {
            return Err(UnixSockApiError::InvalidArgument(
                arg_name.to_string(),
                format!("Invalid argument type: {}", arg_spec.r#type)
            ));
        }
        
        // Validation constraint validation
        if let Some(validation) = &arg_spec.validation {
            Self::validate_validation_spec(arg_name, validation)?;
        }
        
        // Default value type validation
        if let Some(default_value) = &arg_spec.default_value {
            Self::validate_value_type(arg_name, default_value, &arg_spec.r#type)?;
        }
        
        Ok(())
    }
    
    /// Validate validation constraints
    fn validate_validation_spec(
        arg_name: &str,
        validation_spec: &crate::specification::ValidationSpec,
    ) -> Result<(), UnixSockApiError> {
        // Range validation
        if let (Some(min), Some(max)) = (validation_spec.minimum, validation_spec.maximum) {
            if min > max {
                return Err(UnixSockApiError::InvalidArgument(
                    arg_name.to_string(),
                    "Minimum value cannot be greater than maximum value".to_string()
                ));
            }
        }
        
        // Length validation
        if let (Some(min_len), Some(max_len)) = (validation_spec.min_length, validation_spec.max_length) {
            if min_len > max_len {
                return Err(UnixSockApiError::InvalidArgument(
                    arg_name.to_string(),
                    "Minimum length cannot be greater than maximum length".to_string()
                ));
            }
        }
        
        // Pattern validation
        if let Some(pattern) = &validation_spec.pattern {
            regex::Regex::new(pattern)
                .map_err(|e| UnixSockApiError::InvalidArgument(
                    arg_name.to_string(),
                    format!("Invalid regex pattern: {}", e)
                ))?;
        }
        
        // Enum validation
        if let Some(enum_values) = &validation_spec.r#enum {
            if enum_values.is_empty() {
                return Err(UnixSockApiError::InvalidArgument(
                    arg_name.to_string(),
                    "Enum values cannot be empty".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate response specification
    fn validate_response_spec(
        response_spec: &crate::specification::ResponseSpec,
    ) -> Result<(), UnixSockApiError> {
        // Response type validation
        let valid_types = ["string", "integer", "number", "boolean", "array", "object"];
        if !valid_types.contains(&response_spec.r#type.as_str()) {
            return Err(UnixSockApiError::MalformedData(
                format!("Invalid response type: {}", response_spec.r#type)
            ));
        }
        
        // Validate properties if object type
        if response_spec.r#type == "object" {
            if let Some(properties) = &response_spec.properties {
                for (prop_name, prop_spec) in properties {
                    Self::validate_argument_spec(prop_name, prop_spec)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate error code specification
    fn validate_error_code_spec(
        error_name: &str,
        error_spec: &crate::specification::ErrorCodeSpec,
    ) -> Result<(), UnixSockApiError> {
        if error_name.is_empty() {
            return Err(UnixSockApiError::MalformedData(
                "Error code name cannot be empty".to_string()
            ));
        }
        
        if error_spec.message.is_empty() {
            return Err(UnixSockApiError::MalformedData(
                format!("Error code '{}' must have a message", error_name)
            ));
        }
        
        // Validate HTTP status code range
        if !(100..=599).contains(&error_spec.code) {
            return Err(UnixSockApiError::MalformedData(
                format!("Invalid HTTP status code: {}", error_spec.code)
            ));
        }
        
        Ok(())
    }
    
    /// Validate model specification
    fn validate_model(
        model_name: &str,
        model_spec: &crate::specification::ModelSpec,
        _api_spec: &ApiSpecification,
    ) -> Result<(), UnixSockApiError> {
        // Model name validation
        if model_name.is_empty() {
            return Err(UnixSockApiError::MalformedData(
                "Model name cannot be empty".to_string()
            ));
        }
        
        // Validate properties
        for (prop_name, prop_spec) in &model_spec.properties {
            Self::validate_argument_spec(prop_name, prop_spec)?;
        }
        
        // Validate required fields exist
        if let Some(required_fields) = &model_spec.required {
            for required_field in required_fields {
                if !model_spec.properties.contains_key(required_field) {
                    return Err(UnixSockApiError::MalformedData(
                        format!("Required field '{}' not found in model '{}'", 
                               required_field, model_name)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate value matches declared type
    fn validate_value_type(
        arg_name: &str,
        value: &serde_json::Value,
        expected_type: &str,
    ) -> Result<(), UnixSockApiError> {
        let matches = match expected_type {
            "string" => value.is_string(),
            "integer" => value.is_i64(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            _ => false,
        };
        
        if !matches {
            return Err(UnixSockApiError::InvalidArgument(
                arg_name.to_string(),
                format!("Value type mismatch: expected {}, got {}", 
                       expected_type, Self::get_value_type_name(value))
            ));
        }
        
        Ok(())
    }
    
    /// Get JSON value type name
    fn get_value_type_name(value: &serde_json::Value) -> &'static str {
        match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specification::{ChannelSpec, CommandSpec, ArgumentSpec, ResponseSpec, ValidationSpec};
    
    fn create_test_api_spec() -> ApiSpecification {
        let mut api_spec = ApiSpecification::new("1.0.0".to_string());
        
        let mut channel = ChannelSpec::new("Test channel".to_string());
        
        let mut command = CommandSpec::new(
            "Test command".to_string(),
            ResponseSpec::new("object".to_string())
        );
        
        let arg = ArgumentSpec::new("string".to_string())
            .required()
            .with_validation(
                ValidationSpec::new()
                    .with_length_range(Some(1), Some(100))
            );
        
        command.add_argument("test_arg".to_string(), arg);
        channel.add_command("test_cmd".to_string(), command);
        api_spec.add_channel("test_channel".to_string(), channel);
        
        api_spec
    }
    
    #[test]
    fn test_json_serialization() {
        let api_spec = create_test_api_spec();
        
        let json_str = ApiSpecificationParser::to_json(&api_spec).unwrap();
        assert!(!json_str.is_empty());
        
        let parsed_spec = ApiSpecificationParser::from_json(&json_str).unwrap();
        assert_eq!(parsed_spec.version, api_spec.version);
        assert_eq!(parsed_spec.channels.len(), api_spec.channels.len());
    }
    
    #[cfg(feature = "yaml-support")]
    #[test]
    fn test_yaml_serialization() {
        let api_spec = create_test_api_spec();
        
        let yaml_str = ApiSpecificationParser::to_yaml(&api_spec).unwrap();
        assert!(!yaml_str.is_empty());
        
        let parsed_spec = ApiSpecificationParser::from_yaml(&yaml_str).unwrap();
        assert_eq!(parsed_spec.version, api_spec.version);
        assert_eq!(parsed_spec.channels.len(), api_spec.channels.len());
    }
    
    #[test]
    fn test_validation_success() {
        let api_spec = create_test_api_spec();
        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validation_empty_version() {
        let mut api_spec = create_test_api_spec();
        api_spec.version = String::new();
        
        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validation_invalid_version() {
        let mut api_spec = create_test_api_spec();
        api_spec.version = "invalid".to_string();
        
        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validation_empty_channels() {
        let mut api_spec = create_test_api_spec();
        api_spec.channels.clear();
        
        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_version_validation() {
        assert!(ApiSpecificationParser::is_valid_version("1.0.0"));
        assert!(ApiSpecificationParser::is_valid_version("10.20.30"));
        assert!(!ApiSpecificationParser::is_valid_version("1.0"));
        assert!(!ApiSpecificationParser::is_valid_version("1.0.0.0"));
        assert!(!ApiSpecificationParser::is_valid_version("v1.0.0"));
        assert!(!ApiSpecificationParser::is_valid_version("1.0.0-beta"));
    }
    
    #[test]
    fn test_malformed_json() {
        let malformed_json = r#"{"version": "1.0.0", "channels": }"#;
        let result = ApiSpecificationParser::from_json(malformed_json);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_value_type_validation() {
        let string_value = serde_json::Value::String("test".to_string());
        let number_value = serde_json::Value::Number(serde_json::Number::from(42));
        
        assert!(ApiSpecificationParser::validate_value_type("test", &string_value, "string").is_ok());
        assert!(ApiSpecificationParser::validate_value_type("test", &number_value, "number").is_ok());
        assert!(ApiSpecificationParser::validate_value_type("test", &string_value, "number").is_err());
        assert!(ApiSpecificationParser::validate_value_type("test", &number_value, "string").is_err());
    }
}