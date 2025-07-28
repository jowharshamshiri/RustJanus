use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API specification structure (exact SwiftUnixSockAPI parity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiSpecification {
    /// API version
    pub version: String,
    
    /// Channel definitions
    pub channels: HashMap<String, ChannelSpec>,
    
    /// Model definitions (optional)
    pub models: Option<HashMap<String, ModelSpec>>,
}

impl ApiSpecification {
    /// Create a new API specification
    pub fn new(version: String) -> Self {
        Self {
            version,
            channels: HashMap::new(),
            models: None,
        }
    }
    
    /// Load API specification from file (async wrapper)
    pub async fn from_file(path: &str) -> Result<Self, crate::error::UnixSockApiError> {
        crate::specification::ApiSpecificationParser::from_file(path).await
    }
    
    /// Add a channel to the specification
    pub fn add_channel(&mut self, name: String, channel: ChannelSpec) {
        self.channels.insert(name, channel);
    }
    
    /// Add a model to the specification
    pub fn add_model(&mut self, name: String, model: ModelSpec) {
        if self.models.is_none() {
            self.models = Some(HashMap::new());
        }
        self.models.as_mut().unwrap().insert(name, model);
    }
    
    /// Get channel by name
    pub fn get_channel(&self, name: &str) -> Option<&ChannelSpec> {
        self.channels.get(name)
    }
    
    /// Get command specification
    pub fn get_command_spec(&self, channel_id: &str, command_name: &str) -> Option<&CommandSpec> {
        self.channels.get(channel_id)?
            .commands.get(command_name)
    }
    
    /// Get model by name
    pub fn get_model(&self, name: &str) -> Option<&ModelSpec> {
        self.models.as_ref()?.get(name)
    }
    
    /// List all channel names
    pub fn channel_names(&self) -> Vec<&String> {
        self.channels.keys().collect()
    }
    
    /// List all commands in a channel
    pub fn channel_commands(&self, channel_id: &str) -> Option<Vec<&String>> {
        Some(self.channels.get(channel_id)?.commands.keys().collect())
    }
    
    /// Check if command exists in channel
    pub fn has_command(&self, channel_id: &str, command_name: &str) -> bool {
        self.get_command_spec(channel_id, command_name).is_some()
    }
    
    /// Check if channel exists
    pub fn has_channel(&self, channel_id: &str) -> bool {
        self.channels.contains_key(channel_id)
    }
}

/// Channel specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelSpec {
    /// Channel description
    pub description: String,
    
    /// Command definitions
    pub commands: HashMap<String, CommandSpec>,
}

impl ChannelSpec {
    /// Create a new channel specification
    pub fn new(description: String) -> Self {
        Self {
            description,
            commands: HashMap::new(),
        }
    }
    
    /// Add a command to the channel
    pub fn add_command(&mut self, name: String, command: CommandSpec) {
        self.commands.insert(name, command);
    }
    
    /// Get command by name
    pub fn get_command(&self, name: &str) -> Option<&CommandSpec> {
        self.commands.get(name)
    }
    
    /// List all command names
    pub fn command_names(&self) -> Vec<&String> {
        self.commands.keys().collect()
    }
}

/// Command specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandSpec {
    /// Command description
    pub description: String,
    
    /// Argument definitions
    pub args: HashMap<String, ArgumentSpec>,
    
    /// Response specification
    pub response: ResponseSpec,
    
    /// Error code definitions (optional)
    pub error_codes: Option<HashMap<String, ErrorCodeSpec>>,
}

impl CommandSpec {
    /// Create a new command specification
    pub fn new(description: String, response: ResponseSpec) -> Self {
        Self {
            description,
            args: HashMap::new(),
            response,
            error_codes: None,
        }
    }
    
    /// Add an argument to the command
    pub fn add_argument(&mut self, name: String, arg_spec: ArgumentSpec) {
        self.args.insert(name, arg_spec);
    }
    
    /// Add an error code definition
    pub fn add_error_code(&mut self, name: String, error_spec: ErrorCodeSpec) {
        if self.error_codes.is_none() {
            self.error_codes = Some(HashMap::new());
        }
        self.error_codes.as_mut().unwrap().insert(name, error_spec);
    }
    
    /// Get argument specification
    pub fn get_argument(&self, name: &str) -> Option<&ArgumentSpec> {
        self.args.get(name)
    }
    
    /// Get required arguments
    pub fn required_arguments(&self) -> Vec<&String> {
        self.args.iter()
            .filter(|(_, spec)| spec.required.unwrap_or(false))
            .map(|(name, _)| name)
            .collect()
    }
    
    /// Get optional arguments
    pub fn optional_arguments(&self) -> Vec<&String> {
        self.args.iter()
            .filter(|(_, spec)| !spec.required.unwrap_or(false))
            .map(|(name, _)| name)
            .collect()
    }
}

/// Argument specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArgumentSpec {
    /// Argument type (string, integer, number, boolean, array, object)
    pub r#type: String,
    
    /// Whether argument is required (optional, default: false)
    pub required: Option<bool>,
    
    /// Argument description (optional)
    pub description: Option<String>,
    
    /// Default value (optional)
    pub default_value: Option<serde_json::Value>,
    
    /// Validation constraints (optional)
    pub validation: Option<ValidationSpec>,
}

impl ArgumentSpec {
    /// Create a new argument specification
    pub fn new(arg_type: String) -> Self {
        Self {
            r#type: arg_type,
            required: None,
            description: None,
            default_value: None,
            validation: None,
        }
    }
    
    /// Set as required argument
    pub fn required(mut self) -> Self {
        self.required = Some(true);
        self
    }
    
    /// Set as optional argument
    pub fn optional(mut self) -> Self {
        self.required = Some(false);
        self
    }
    
    /// Add description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    
    /// Add default value
    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default_value = Some(value);
        self
    }
    
    /// Add validation constraints
    pub fn with_validation(mut self, validation: ValidationSpec) -> Self {
        self.validation = Some(validation);
        self
    }
    
    /// Check if argument is required
    pub fn is_required(&self) -> bool {
        self.required.unwrap_or(false)
    }
    
    /// Check if argument has default value
    pub fn has_default(&self) -> bool {
        self.default_value.is_some()
    }
}

/// Validation constraint specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationSpec {
    /// Minimum length for strings/arrays (optional)
    pub min_length: Option<usize>,
    
    /// Maximum length for strings/arrays (optional)
    pub max_length: Option<usize>,
    
    /// Regular expression pattern for strings (optional)
    pub pattern: Option<String>,
    
    /// Minimum value for numbers (optional)
    pub minimum: Option<f64>,
    
    /// Maximum value for numbers (optional)
    pub maximum: Option<f64>,
    
    /// Enumerated allowed values (optional)
    pub r#enum: Option<Vec<serde_json::Value>>,
}

impl ValidationSpec {
    /// Create a new validation specification
    pub fn new() -> Self {
        Self {
            min_length: None,
            max_length: None,
            pattern: None,
            minimum: None,
            maximum: None,
            r#enum: None,
        }
    }
    
    /// Set length constraints
    pub fn with_length_range(mut self, min: Option<usize>, max: Option<usize>) -> Self {
        self.min_length = min;
        self.max_length = max;
        self
    }
    
    /// Set numeric range constraints
    pub fn with_numeric_range(mut self, min: Option<f64>, max: Option<f64>) -> Self {
        self.minimum = min;
        self.maximum = max;
        self
    }
    
    /// Set pattern constraint
    pub fn with_pattern(mut self, pattern: String) -> Self {
        self.pattern = Some(pattern);
        self
    }
    
    /// Set enumerated values
    pub fn with_enum(mut self, values: Vec<serde_json::Value>) -> Self {
        self.r#enum = Some(values);
        self
    }
}

impl Default for ValidationSpec {
    fn default() -> Self {
        Self::new()
    }
}

/// Response specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseSpec {
    /// Response type
    pub r#type: String,
    
    /// Response properties for object types (optional)
    pub properties: Option<HashMap<String, ArgumentSpec>>,
}

impl ResponseSpec {
    /// Create a new response specification
    pub fn new(response_type: String) -> Self {
        Self {
            r#type: response_type,
            properties: None,
        }
    }
    
    /// Add properties for object response
    pub fn with_properties(mut self, properties: HashMap<String, ArgumentSpec>) -> Self {
        self.properties = Some(properties);
        self
    }
    
    /// Add a single property
    pub fn add_property(&mut self, name: String, property: ArgumentSpec) {
        if self.properties.is_none() {
            self.properties = Some(HashMap::new());
        }
        self.properties.as_mut().unwrap().insert(name, property);
    }
}

/// Error code specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorCodeSpec {
    /// HTTP-style error code
    pub code: u16,
    
    /// Error message
    pub message: String,
    
    /// Detailed error description (optional)
    pub description: Option<String>,
}

impl ErrorCodeSpec {
    /// Create a new error code specification
    pub fn new(code: u16, message: String) -> Self {
        Self {
            code,
            message,
            description: None,
        }
    }
    
    /// Add description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

/// Model specification for complex data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSpec {
    /// Model type (typically "object")
    pub r#type: String,
    
    /// Property definitions
    pub properties: HashMap<String, ArgumentSpec>,
    
    /// Required property names (optional)
    pub required: Option<Vec<String>>,
}

impl ModelSpec {
    /// Create a new model specification
    pub fn new() -> Self {
        Self {
            r#type: "object".to_string(),
            properties: HashMap::new(),
            required: None,
        }
    }
    
    /// Add a property to the model
    pub fn add_property(&mut self, name: String, property: ArgumentSpec) {
        self.properties.insert(name, property);
    }
    
    /// Set required properties
    pub fn with_required(mut self, required: Vec<String>) -> Self {
        self.required = Some(required);
        self
    }
    
    /// Add a required property
    pub fn add_required(&mut self, property_name: String) {
        if self.required.is_none() {
            self.required = Some(Vec::new());
        }
        self.required.as_mut().unwrap().push(property_name);
    }
    
    /// Check if property is required
    pub fn is_property_required(&self, property_name: &str) -> bool {
        self.required.as_ref()
            .map(|req| req.contains(&property_name.to_string()))
            .unwrap_or(false)
    }
    
    /// Get required properties
    pub fn required_properties(&self) -> Vec<&String> {
        self.required.as_ref()
            .map(|req| req.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for ModelSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_specification_creation() {
        let mut api_spec = ApiSpecification::new("1.0.0".to_string());
        
        let channel = ChannelSpec::new("Test channel".to_string());
        api_spec.add_channel("test".to_string(), channel);
        
        assert_eq!(api_spec.version, "1.0.0");
        assert_eq!(api_spec.channels.len(), 1);
        assert!(api_spec.get_channel("test").is_some());
    }
    
    #[test]
    fn test_channel_specification() {
        let mut channel = ChannelSpec::new("Test channel".to_string());
        
        let response = ResponseSpec::new("object".to_string());
        let command = CommandSpec::new("Test command".to_string(), response);
        channel.add_command("test-cmd".to_string(), command);
        
        assert_eq!(channel.description, "Test channel");
        assert_eq!(channel.commands.len(), 1);
        assert!(channel.get_command("test-cmd").is_some());
    }
    
    #[test]
    fn test_argument_specification() {
        let arg_spec = ArgumentSpec::new("string".to_string())
            .required()
            .with_description("Test argument".to_string())
            .with_validation(
                ValidationSpec::new()
                    .with_length_range(Some(1), Some(100))
                    .with_pattern("^[a-zA-Z]+$".to_string())
            );
        
        assert_eq!(arg_spec.r#type, "string");
        assert!(arg_spec.is_required());
        assert_eq!(arg_spec.description, Some("Test argument".to_string()));
        assert!(arg_spec.validation.is_some());
    }
    
    #[test]
    fn test_validation_specification() {
        let validation = ValidationSpec::new()
            .with_length_range(Some(5), Some(50))
            .with_numeric_range(Some(0.0), Some(100.0))
            .with_pattern("^test.*".to_string())
            .with_enum(vec![
                serde_json::Value::String("option1".to_string()),
                serde_json::Value::String("option2".to_string()),
            ]);
        
        assert_eq!(validation.min_length, Some(5));
        assert_eq!(validation.max_length, Some(50));
        assert_eq!(validation.minimum, Some(0.0));
        assert_eq!(validation.maximum, Some(100.0));
        assert!(validation.pattern.is_some());
        assert!(validation.r#enum.is_some());
    }
    
    #[test]
    fn test_command_specification() {
        let response = ResponseSpec::new("object".to_string());
        let mut command = CommandSpec::new("Test command".to_string(), response);
        
        let arg = ArgumentSpec::new("string".to_string()).required();
        command.add_argument("test_arg".to_string(), arg);
        
        let error_code = ErrorCodeSpec::new(400, "Bad Request".to_string());
        command.add_error_code("bad_request".to_string(), error_code);
        
        assert_eq!(command.description, "Test command");
        assert_eq!(command.args.len(), 1);
        assert_eq!(command.required_arguments().len(), 1);
        assert!(command.error_codes.is_some());
    }
    
    #[test]
    fn test_model_specification() {
        let mut model = ModelSpec::new()
            .with_required(vec!["name".to_string(), "email".to_string()]);
        
        let name_prop = ArgumentSpec::new("string".to_string()).required();
        let email_prop = ArgumentSpec::new("string".to_string()).required();
        let age_prop = ArgumentSpec::new("integer".to_string()).optional();
        
        model.add_property("name".to_string(), name_prop);
        model.add_property("email".to_string(), email_prop);
        model.add_property("age".to_string(), age_prop);
        
        assert_eq!(model.properties.len(), 3);
        assert!(model.is_property_required("name"));
        assert!(model.is_property_required("email"));
        assert!(!model.is_property_required("age"));
        assert_eq!(model.required_properties().len(), 2);
    }
    
    #[test]
    fn test_api_spec_command_lookup() {
        let mut api_spec = ApiSpecification::new("1.0.0".to_string());
        
        let mut channel = ChannelSpec::new("Test channel".to_string());
        let response = ResponseSpec::new("string".to_string());
        let command = CommandSpec::new("Test command".to_string(), response);
        channel.add_command("test-cmd".to_string(), command);
        
        api_spec.add_channel("test-channel".to_string(), channel);
        
        assert!(api_spec.has_command("test-channel", "test-cmd"));
        assert!(!api_spec.has_command("test-channel", "nonexistent"));
        assert!(!api_spec.has_command("nonexistent", "test-cmd"));
        
        let cmd_spec = api_spec.get_command_spec("test-channel", "test-cmd");
        assert!(cmd_spec.is_some());
        assert_eq!(cmd_spec.unwrap().description, "Test command");
    }
}