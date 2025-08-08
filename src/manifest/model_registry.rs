use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manifest structure (exact SwiftJanus parity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// API version
    pub version: String,
    
    /// Model definitions (optional)
    pub models: Option<HashMap<String, ModelManifest>>,
}

impl Manifest {
    /// Create a new Manifest
    pub fn new(version: String) -> Self {
        Self {
            version,
            models: None,
        }
    }
    
    /// Load Manifest from file (async wrapper)
    pub async fn from_file(path: &str) -> Result<Self, crate::error::JSONRPCError> {
        crate::manifest::ManifestParser::from_file(path).await
    }
    
    /// Add a model to the manifest
    pub fn add_model(&mut self, name: String, model: ModelManifest) {
        if self.models.is_none() {
            self.models = Some(HashMap::new());
        }
        self.models.as_mut().unwrap().insert(name, model);
    }
    
    /// Get model by name
    pub fn get_model(&self, name: &str) -> Option<&ModelManifest> {
        self.models.as_ref()?.get(name)
    }
    
    /// Check if request exists (channels removed from protocol)
    pub fn has_request(&self, request_name: &str) -> bool {
        // Since channels are removed, this always returns false for now
        // The server will handle request validation
        false
    }
    
    /// Get request manifest (channels removed from protocol)
    pub fn get_request_manifest(&self, request_name: &str) -> Option<&RequestManifest> {
        // Since channels are removed, this always returns None
        // The server will handle request validation
        None
    }
}


/// Request manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequestManifest {
    /// Request name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Request description
    pub description: String,
    
    /// Argument definitions
    #[serde(default)]
    pub args: HashMap<String, ArgumentManifest>,
    
    /// Response manifest
    pub response: ResponseManifest,
    
    /// Error code definitions (optional)
    pub error_codes: Option<HashMap<String, ErrorCodeManifest>>,
}

impl RequestManifest {
    /// Create a new request manifest
    pub fn new(description: String, response: ResponseManifest) -> Self {
        Self {
            name: None,
            description,
            args: HashMap::new(),
            response,
            error_codes: None,
        }
    }
    
    /// Add an argument to the request
    pub fn add_argument(&mut self, name: String, arg_manifest: ArgumentManifest) {
        self.args.insert(name, arg_manifest);
    }
    
    /// Add an error code definition
    pub fn add_error_code(&mut self, name: String, error_manifest: ErrorCodeManifest) {
        if self.error_codes.is_none() {
            self.error_codes = Some(HashMap::new());
        }
        self.error_codes.as_mut().unwrap().insert(name, error_manifest);
    }
    
    /// Get argument manifest
    pub fn get_argument(&self, name: &str) -> Option<&ArgumentManifest> {
        self.args.get(name)
    }
    
    /// Get required arguments
    pub fn required_arguments(&self) -> Vec<&String> {
        self.args.iter()
            .filter(|(_, manifest)| manifest.required.unwrap_or(false))
            .map(|(name, _)| name)
            .collect()
    }
    
    /// Get optional arguments
    pub fn optional_arguments(&self) -> Vec<&String> {
        self.args.iter()
            .filter(|(_, manifest)| !manifest.required.unwrap_or(false))
            .map(|(name, _)| name)
            .collect()
    }
}

/// Argument manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArgumentManifest {
    /// Argument type (string, integer, number, boolean, array, object)
    pub r#type: String,
    
    /// Whether argument is required (optional, default: false)
    pub required: Option<bool>,
    
    /// Argument description (optional)
    pub description: Option<String>,
    
    /// Default value (optional)
    pub default_value: Option<serde_json::Value>,
    
    /// Validation constraints (optional)
    pub validation: Option<ValidationManifest>,
    
    /// Model reference for complex types (optional)
    #[serde(rename = "modelRef")]
    pub model_ref: Option<String>,
}

impl ArgumentManifest {
    /// Create a new argument manifest
    pub fn new(arg_type: String) -> Self {
        Self {
            r#type: arg_type,
            required: None,
            description: None,
            default_value: None,
            validation: None,
            model_ref: None,
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
    pub fn with_validation(mut self, validation: ValidationManifest) -> Self {
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

/// Validation constraint manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationManifest {
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

impl ValidationManifest {
    /// Create a new validation manifest
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

impl Default for ValidationManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseManifest {
    /// Response type
    pub r#type: String,
    
    /// Response properties for object types (optional)
    pub properties: Option<HashMap<String, ArgumentManifest>>,
    
    /// Model reference for complex types (optional)
    #[serde(rename = "modelRef")]
    pub model_ref: Option<String>,
}

impl ResponseManifest {
    /// Create a new response manifest
    pub fn new(response_type: String) -> Self {
        Self {
            r#type: response_type,
            properties: None,
            model_ref: None,
        }
    }
    
    /// Add properties for object response
    pub fn with_properties(mut self, properties: HashMap<String, ArgumentManifest>) -> Self {
        self.properties = Some(properties);
        self
    }
    
    /// Add a single property
    pub fn add_property(&mut self, name: String, property: ArgumentManifest) {
        if self.properties.is_none() {
            self.properties = Some(HashMap::new());
        }
        self.properties.as_mut().unwrap().insert(name, property);
    }
}

/// Error code manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorCodeManifest {
    /// HTTP-style error code
    pub code: u16,
    
    /// Error message
    pub message: String,
    
    /// Detailed error description (optional)
    pub description: Option<String>,
}

impl ErrorCodeManifest {
    /// Create a new error code manifest
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

/// Model manifest for complex data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelManifest {
    /// Model type (typically "object")
    pub r#type: String,
    
    /// Property definitions
    pub properties: HashMap<String, ArgumentManifest>,
    
    /// Required property names (optional)
    pub required: Option<Vec<String>>,
}

impl ModelManifest {
    /// Create a new model manifest
    pub fn new() -> Self {
        Self {
            r#type: "object".to_string(),
            properties: HashMap::new(),
            required: None,
        }
    }
    
    /// Add a property to the model
    pub fn add_property(&mut self, name: String, property: ArgumentManifest) {
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

impl Default for ModelManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_manifest_creation() {
        let manifest = Manifest::new("1.0.0".to_string());
        
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.models.is_none());
    }
    
    
    #[test]
    fn test_argument_manifest() {
        let arg_manifest = ArgumentManifest::new("string".to_string())
            .required()
            .with_description("Test argument".to_string())
            .with_validation(
                ValidationManifest::new()
                    .with_length_range(Some(1), Some(100))
                    .with_pattern("^[a-zA-Z]+$".to_string())
            );
        
        assert_eq!(arg_manifest.r#type, "string");
        assert!(arg_manifest.is_required());
        assert_eq!(arg_manifest.description, Some("Test argument".to_string()));
        assert!(arg_manifest.validation.is_some());
    }
    
    #[test]
    fn test_validation_manifest() {
        let validation = ValidationManifest::new()
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
    fn test_request_manifest() {
        let response = ResponseManifest::new("object".to_string());
        let mut request = RequestManifest::new("Test request".to_string(), response);
        
        let arg = ArgumentManifest::new("string".to_string()).required();
        request.add_argument("test_arg".to_string(), arg);
        
        let error_code = ErrorCodeManifest::new(400, "Bad Request".to_string());
        request.add_error_code("bad_request".to_string(), error_code);
        
        assert_eq!(request.description, "Test request");
        assert_eq!(request.args.len(), 1);
        assert_eq!(request.required_arguments().len(), 1);
        assert!(request.error_codes.is_some());
    }
    
    #[test]
    fn test_model_manifest() {
        let mut model = ModelManifest::new()
            .with_required(vec!["name".to_string(), "email".to_string()]);
        
        let name_prop = ArgumentManifest::new("string".to_string()).required();
        let email_prop = ArgumentManifest::new("string".to_string()).required();
        let age_prop = ArgumentManifest::new("integer".to_string()).optional();
        
        model.add_property("name".to_string(), name_prop);
        model.add_property("email".to_string(), email_prop);
        model.add_property("age".to_string(), age_prop);
        
        assert_eq!(model.properties.len(), 3);
        assert!(model.is_property_required("name"));
        assert!(model.is_property_required("email"));
        assert!(!model.is_property_required("age"));
        assert_eq!(model.required_properties().len(), 2);
    }
    
}