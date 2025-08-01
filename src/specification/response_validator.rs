/*!
 * Response Validator for Rust Janus Implementation
 * Validates command handler responses against API specification ResponseSpec models
 * Achieves 100% parity with TypeScript and Go implementations
 */

use crate::specification::model_registry::{ApiSpecification, ResponseSpec, ArgumentSpec, ModelSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;
use regex::Regex;

/// Represents a validation error with detailed context
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationError {
    /// Field path that failed validation
    pub field: String,
    
    /// Human-readable error message
    pub message: String,
    
    /// Expected type or value
    pub expected: String,
    
    /// Actual value that failed validation
    pub actual: Value,
    
    /// Additional validation context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "validation error for field '{}': {} (expected: {}, actual: {})", 
               self.field, self.message, self.expected, self.actual)
    }
}

impl std::error::Error for ValidationError {}

/// Result of response validation
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    
    /// List of validation errors (empty if valid)
    pub errors: Vec<ValidationError>,
    
    /// Time taken for validation in milliseconds
    pub validation_time: f64,
    
    /// Number of fields validated
    pub fields_validated: usize,
}

/// Response validator that validates command handler responses
/// against API specification ResponseSpec models
pub struct ResponseValidator {
    specification: ApiSpecification,
}

impl ResponseValidator {
    /// Create a new response validator with the given API specification
    pub fn new(specification: ApiSpecification) -> Self {
        Self { specification }
    }

    /// Validate a response against a ResponseSpec
    pub fn validate_response(&self, response: &Value, response_spec: &ResponseSpec) -> ValidationResult {
        let start_time = Instant::now();
        let mut errors = Vec::new();
        
        // Validate the response value against the specification
        self.validate_value(response, &response_spec.into(), "", &mut errors);
        
        let fields_validated = self.count_validated_fields(&response_spec.into());
        let validation_time = start_time.elapsed().as_secs_f64() * 1000.0; // Convert to milliseconds
        
        ValidationResult {
            valid: errors.is_empty(),
            errors,
            validation_time,
            fields_validated,
        }
    }

    /// Validate a command response by looking up the command specification
    pub fn validate_command_response(&self, response: &Value, channel_id: &str, command_name: &str) -> ValidationResult {
        let start_time = Instant::now();
        
        // Look up command specification
        let channel = match self.specification.channels.get(channel_id) {
            Some(channel) => channel,
            None => {
                return ValidationResult {
                    valid: false,
                    errors: vec![ValidationError {
                        field: "channelId".to_string(),
                        message: format!("Channel '{}' not found in API specification", channel_id),
                        expected: "valid channel ID".to_string(),
                        actual: Value::String(channel_id.to_string()),
                        context: None,
                    }],
                    validation_time: start_time.elapsed().as_secs_f64() * 1000.0,
                    fields_validated: 0,
                };
            }
        };
        
        let command = match channel.commands.get(command_name) {
            Some(command) => command,
            None => {
                return ValidationResult {
                    valid: false,
                    errors: vec![ValidationError {
                        field: "command".to_string(),
                        message: format!("Command '{}' not found in channel '{}'", command_name, channel_id),
                        expected: "valid command name".to_string(),
                        actual: Value::String(command_name.to_string()),
                        context: None,
                    }],
                    validation_time: start_time.elapsed().as_secs_f64() * 1000.0,
                    fields_validated: 0,
                };
            }
        };
        
        // Response is not optional in Rust implementation
        let response_spec = &command.response;
        
        self.validate_response(response, response_spec)
    }

    /// Validate a value against a specification
    fn validate_value(&self, value: &Value, spec: &SpecType, field_path: &str, errors: &mut Vec<ValidationError>) {
        // Handle model references first
        match spec {
            SpecType::Response(response_spec) => {
                if let Some(model_ref) = &response_spec.model_ref {
                    if let Some(model) = self.resolve_model_reference(model_ref) {
                        self.validate_value(value, &SpecType::Model(model.clone()), field_path, errors);
                        return;
                    } else {
                        errors.push(ValidationError {
                            field: field_path.to_string(),
                            message: format!("Model reference '{}' not found", model_ref),
                            expected: "valid model reference".to_string(),
                            actual: Value::String(model_ref.clone()),
                            context: None,
                        });
                        return;
                    }
                }
            }
            SpecType::Argument(arg_spec) => {
                if let Some(model_ref) = &arg_spec.model_ref {
                    if let Some(model) = self.resolve_model_reference(model_ref) {
                        self.validate_value(value, &SpecType::Model(model.clone()), field_path, errors);
                        return;
                    } else {
                        errors.push(ValidationError {
                            field: field_path.to_string(),
                            message: format!("Model reference '{}' not found", model_ref),
                            expected: "valid model reference".to_string(),
                            actual: Value::String(model_ref.clone()),
                            context: None,
                        });
                        return;
                    }
                }
            }
            SpecType::Model(_) => {
                // No model reference needed for already resolved models
            }
        }

        // Validate type
        let initial_error_count = errors.len();
        self.validate_type(value, spec.type_str(), field_path, errors);

        if errors.len() > initial_error_count {
            return; // Don't continue validation if type is wrong
        }

        // Type-specific validation
        match spec.type_str() {
            "string" => {
                if let Value::String(str_value) = value {
                    self.validate_string(str_value, spec, field_path, errors);
                }
            }
            "number" | "integer" => {
                if let Some(num_value) = self.get_numeric_value(value) {
                    self.validate_number(num_value, spec.type_str(), spec, field_path, errors);
                }
            }
            "array" => {
                if let Value::Array(array_value) = value {
                    self.validate_array(array_value, spec, field_path, errors);
                }
            }
            "object" => {
                if let Value::Object(obj_value) = value {
                    self.validate_object(obj_value, spec, field_path, errors);
                }
            }
            "boolean" => {
                // Boolean validation is covered by type validation
            }
            _ => {}
        }

        // Validate enum values (only available on ArgumentSpec through ValidationSpec)
        if let Some(enum_values) = spec.enum_values() {
            self.validate_enum(value, enum_values, field_path, errors);
        }
    }

    /// Validate the type of a value
    fn validate_type(&self, value: &Value, expected_type: &str, field_path: &str, errors: &mut Vec<ValidationError>) {
        let actual_type = self.get_actual_type(value);

        if expected_type == "integer" {
            if actual_type != "number" || !self.is_integer(value) {
                errors.push(ValidationError {
                    field: field_path.to_string(),
                    message: "Value is not an integer".to_string(),
                    expected: "integer".to_string(),
                    actual: Value::String(actual_type.to_string()),
                    context: None,
                });
            }
        } else if actual_type != expected_type {
            errors.push(ValidationError {
                field: field_path.to_string(),
                message: "Type mismatch".to_string(),
                expected: expected_type.to_string(),
                actual: Value::String(actual_type.to_string()),
                context: None,
            });
        }
    }

    /// Get the actual type string of a value
    fn get_actual_type(&self, value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Check if a numeric value is an integer
    fn is_integer(&self, value: &Value) -> bool {
        if let Value::Number(num) = value {
            if let Some(int_val) = num.as_i64() {
                num.as_f64().map_or(false, |f| f == int_val as f64)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Extract a numeric value from JSON value
    fn get_numeric_value(&self, value: &Value) -> Option<f64> {
        if let Value::Number(num) = value {
            num.as_f64()
        } else {
            None
        }
    }

    /// Validate string value constraints
    fn validate_string(&self, value: &str, spec: &SpecType, field_path: &str, errors: &mut Vec<ValidationError>) {
        // Only ArgumentSpec has validation constraints in Rust implementation
        if let SpecType::Argument(arg_spec) = spec {
            if let Some(ref validation) = arg_spec.validation {
                // Length validation
                if let Some(min_length) = validation.min_length {
                    if value.len() < min_length {
                        errors.push(ValidationError {
                            field: field_path.to_string(),
                            message: format!("String is too short ({} < {})", value.len(), min_length),
                            expected: format!("minimum length {}", min_length),
                            actual: Value::String(format!("length {}", value.len())),
                            context: None,
                        });
                    }
                }

                if let Some(max_length) = validation.max_length {
                    if value.len() > max_length {
                        errors.push(ValidationError {
                            field: field_path.to_string(),
                            message: format!("String is too long ({} > {})", value.len(), max_length),
                            expected: format!("maximum length {}", max_length),
                            actual: Value::String(format!("length {}", value.len())),
                            context: None,
                        });
                    }
                }

                // Pattern validation
                if let Some(ref pattern) = validation.pattern {
                    match Regex::new(pattern) {
                        Ok(regex) => {
                            if !regex.is_match(value) {
                                errors.push(ValidationError {
                                    field: field_path.to_string(),
                                    message: "String does not match required pattern".to_string(),
                                    expected: format!("pattern {}", pattern),
                                    actual: Value::String(value.to_string()),
                                    context: None,
                                });
                            }
                        }
                        Err(_) => {
                            errors.push(ValidationError {
                                field: field_path.to_string(),
                                message: "Invalid regex pattern in specification".to_string(),
                                expected: "valid regex pattern".to_string(),
                                actual: Value::String(pattern.clone()),
                                context: None,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Validate numeric value constraints
    fn validate_number(&self, value: f64, _value_type: &str, spec: &SpecType, field_path: &str, errors: &mut Vec<ValidationError>) {
        // Range validation (only available on ArgumentSpec through ValidationSpec)
        if let SpecType::Argument(arg_spec) = spec {
            if let Some(ref validation) = arg_spec.validation {
                if let Some(minimum) = validation.minimum {
                    if value < minimum {
                        errors.push(ValidationError {
                            field: field_path.to_string(),
                            message: format!("Number is too small ({} < {})", value, minimum),
                            expected: format!("minimum {}", minimum),
                            actual: Value::Number(serde_json::Number::from_f64(value).unwrap()),
                            context: None,
                        });
                    }
                }

                if let Some(maximum) = validation.maximum {
                    if value > maximum {
                        errors.push(ValidationError {
                            field: field_path.to_string(),
                            message: format!("Number is too large ({} > {})", value, maximum),
                            expected: format!("maximum {}", maximum),
                            actual: Value::Number(serde_json::Number::from_f64(value).unwrap()),
                            context: None,
                        });
                    }
                }
            }
        }
    }

    /// Validate array value and items (matches Go implementation)
    fn validate_array(&self, value: &[Value], spec: &SpecType, field_path: &str, errors: &mut Vec<ValidationError>) {
        // Basic array validation - type checking is handled by caller
        
        // For each array item, validate recursively if we have type information
        // This is a basic implementation matching Go's recursive array validation
        for (index, item) in value.iter().enumerate() {
            let item_path = format!("{}[{}]", field_path, index);
            
            // For arrays, we can do basic type consistency validation
            // More advanced validation would require array item type specs
            match item {
                Value::Object(obj) => {
                    // Validate object items recursively if we have object spec
                    if let SpecType::Response(response_spec) = spec {
                        if let Some(properties) = &response_spec.properties {
                            self.validate_object_properties(obj, properties, &item_path, errors);
                        }
                    }
                }
                Value::Array(arr) => {
                    // Recursive array validation
                    self.validate_array(arr, spec, &item_path, errors);
                }
                _ => {
                    // Basic type validation for primitive array items
                    // Additional constraints could be added here
                }
            }
        }
    }
    
    /// Validate object properties (helper method for array validation)
    fn validate_object_properties(&self, value: &serde_json::Map<String, Value>, properties: &std::collections::HashMap<String, ArgumentSpec>, field_path: &str, errors: &mut Vec<ValidationError>) {
        // Validate each property
        for (prop_name, prop_spec) in properties {
            let prop_field_path = if field_path.is_empty() {
                prop_name.clone()
            } else {
                format!("{}.{}", field_path, prop_name)
            };

            let prop_value = value.get(prop_name);

            // Check required fields
            let is_required = prop_spec.required.unwrap_or(false);
            if is_required && (prop_value.is_none() || prop_value == Some(&Value::Null)) {
                errors.push(ValidationError {
                    field: prop_field_path.clone(),
                    message: "Required field is missing or null".to_string(),
                    expected: format!("non-null {}", prop_spec.r#type),
                    actual: prop_value.cloned().unwrap_or(Value::Null),
                    context: None,
                });
                continue;
            }

            // Validate property value if present
            if let Some(value) = prop_value {
                // Use the property spec directly as ArgumentSpec (they have compatible structure)
                let spec_type = SpecType::Argument(prop_spec.clone());
                self.validate_value(value, &spec_type, &prop_field_path, errors);
            }
        }
    }

    /// Validate object properties
    fn validate_object(&self, value: &serde_json::Map<String, Value>, spec: &SpecType, field_path: &str, errors: &mut Vec<ValidationError>) {
        let properties = match spec {
            SpecType::Response(response_spec) => response_spec.properties.as_ref(),
            SpecType::Model(model_spec) => Some(&model_spec.properties),
            SpecType::Argument(_arg_spec) => {
                // ArgumentSpec doesn't have properties in current Rust implementation
                return;
            }
        };

        if let Some(properties) = properties {
            // Validate each property
            for (prop_name, prop_spec) in properties {
                let prop_field_path = if field_path.is_empty() {
                    prop_name.clone()
                } else {
                    format!("{}.{}", field_path, prop_name)
                };

                let prop_value = value.get(prop_name);

                // Check required fields
                let is_required = prop_spec.required.unwrap_or(false);
                if is_required && (prop_value.is_none() || prop_value == Some(&Value::Null)) {
                    errors.push(ValidationError {
                        field: prop_field_path,
                        message: "Required field is missing or null".to_string(),
                        expected: format!("non-null {}", prop_spec.r#type),
                        actual: prop_value.cloned().unwrap_or(Value::Null),
                        context: None,
                    });
                    continue;
                }

                // Skip validation for optional missing fields
                if prop_value.is_none() && !is_required {
                    continue;
                }

                // Validate property value
                if let Some(prop_val) = prop_value {
                    self.validate_value(prop_val, &SpecType::Argument(prop_spec.clone()), &prop_field_path, errors);
                }
            }
        }
    }

    /// Validate enum constraints
    fn validate_enum(&self, value: &Value, enum_values: &[Value], field_path: &str, errors: &mut Vec<ValidationError>) {
        if !enum_values.contains(value) {
            let enum_strings: Vec<String> = enum_values.iter().map(|v| v.to_string()).collect();
            errors.push(ValidationError {
                field: field_path.to_string(),
                message: "Value is not in allowed enum list".to_string(),
                expected: enum_strings.join(", "),
                actual: value.clone(),
                context: None,
            });
        }
    }

    /// Resolve a model reference to its definition
    fn resolve_model_reference(&self, model_ref: &str) -> Option<&ModelSpec> {
        self.specification.models.as_ref()?.get(model_ref)
    }

    /// Count the number of fields that would be validated
    fn count_validated_fields(&self, spec: &SpecType) -> usize {
        if spec.type_str() == "object" {
            match spec {
                SpecType::Response(response_spec) => {
                    response_spec.properties.as_ref().map_or(1, |props| props.len())
                }
                SpecType::Model(model_spec) => model_spec.properties.len(),
                SpecType::Argument(_) => 1,
            }
        } else {
            1
        }
    }

    /// Create a validation error for missing response specification
    pub fn create_missing_specification_error(channel_id: &str, command_name: &str) -> ValidationResult {
        ValidationResult {
            valid: false,
            errors: vec![ValidationError {
                field: "specification".to_string(),
                message: format!("No response specification found for command '{}' in channel '{}'", command_name, channel_id),
                expected: "response specification".to_string(),
                actual: Value::String("undefined".to_string()),
                context: None,
            }],
            validation_time: 0.0,
            fields_validated: 0,
        }
    }

    /// Create a validation result for successful validation
    pub fn create_success_result(fields_validated: usize, validation_time: f64) -> ValidationResult {
        ValidationResult {
            valid: true,
            errors: Vec::new(),
            validation_time,
            fields_validated,
        }
    }
}

/// Unified specification type for validation
#[derive(Clone)]
enum SpecType {
    Response(ResponseSpec),
    Argument(ArgumentSpec),
    Model(ModelSpec),
}

impl SpecType {
    fn type_str(&self) -> &str {
        match self {
            SpecType::Response(spec) => &spec.r#type,
            SpecType::Argument(spec) => &spec.r#type,
            SpecType::Model(spec) => &spec.r#type,
        }
    }

    fn enum_values(&self) -> Option<&[Value]> {
        match self {
            SpecType::Argument(spec) => {
                spec.validation.as_ref()?.r#enum.as_deref()
            },
            _ => None,
        }
    }
}

impl From<&ResponseSpec> for SpecType {
    fn from(spec: &ResponseSpec) -> Self {
        SpecType::Response(spec.clone())
    }
}

impl From<&ArgumentSpec> for SpecType {
    fn from(spec: &ArgumentSpec) -> Self {
        SpecType::Argument(spec.clone())
    }
}

impl From<&ModelSpec> for SpecType {
    fn from(spec: &ModelSpec) -> Self {
        SpecType::Model(spec.clone())
    }
}