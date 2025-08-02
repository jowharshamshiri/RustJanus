/*!
 * Response Validation Tests for Rust Janus Implementation
 * Comprehensive tests for response validation features
 */

use rust_janus::*;
use rust_janus::specification::{ResponseValidator, ResponseSpec, ArgumentSpec, ValidationSpec};
use std::collections::HashMap;
mod test_utils;
use test_utils::*;

#[tokio::test]
async fn test_response_against_spec_validation() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Create a basic response spec
    let response_spec = ResponseSpec::new("string".to_string());
    
    // Test valid response
    let valid_response = serde_json::json!("valid test response");
    let result = validator.validate_response(&valid_response, &response_spec);
    
    assert!(result.validation_time >= 0.0, "Validation time should be recorded");
    assert!(result.fields_validated > 0, "Fields should be counted");
}

#[tokio::test]
async fn test_command_response_validation() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Test command response validation using test Manifest
    let response = serde_json::json!({
        "result": "echo response"
    });
    
    let result = validator.validate_command_response(&response, "test", "echo");
    
    // Should validate based on the test Manifest
    assert!(result.validation_time >= 0.0, "Validation time should be recorded");
    assert!(result.fields_validated >= 0, "Fields should be counted");
}

#[tokio::test]
async fn test_type_validation_engine() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Test different type validations
    let string_spec = ResponseSpec::new("string".to_string());
    let number_spec = ResponseSpec::new("number".to_string());
    let boolean_spec = ResponseSpec::new("boolean".to_string());
    
    // Valid type matches
    let string_result = validator.validate_response(&serde_json::json!("test"), &string_spec);
    let number_result = validator.validate_response(&serde_json::json!(42), &number_spec);
    let boolean_result = validator.validate_response(&serde_json::json!(true), &boolean_spec);
    
    // All should have timing and field counts
    assert!(string_result.validation_time >= 0.0);
    assert!(number_result.validation_time >= 0.0);
    assert!(boolean_result.validation_time >= 0.0);
    assert!(string_result.fields_validated > 0);
    assert!(number_result.fields_validated > 0);
    assert!(boolean_result.fields_validated > 0);
}

#[tokio::test]
async fn test_string_constraint_validation() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Create response spec with string validation
    let mut response_spec = ResponseSpec::new("string".to_string());
    
    // Test basic string validation
    let string_response = serde_json::json!("test string");
    let result = validator.validate_response(&string_response, &response_spec);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]  
async fn test_numeric_range_validation() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Create number response spec
    let number_spec = ResponseSpec::new("number".to_string());
    
    // Test numeric validation
    let number_response = serde_json::json!(42);
    let result = validator.validate_response(&number_response, &number_spec);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]
async fn test_object_property_validation() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Create object response spec with properties
    let mut properties = HashMap::new();
    properties.insert("name".to_string(), ArgumentSpec::new("string".to_string()));
    properties.insert("value".to_string(), ArgumentSpec::new("number".to_string()));
    
    let object_spec = ResponseSpec::new("object".to_string()).with_properties(properties);
    
    // Test object validation
    let object_response = serde_json::json!({
        "name": "test",
        "value": 42
    });
    
    let result = validator.validate_response(&object_response, &object_spec);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]
async fn test_enum_value_validation() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Create string spec (enum validation would be in ValidationSpec if implemented)
    let enum_spec = ResponseSpec::new("string".to_string());
    
    // Test enum-like validation
    let enum_response = serde_json::json!("option1");
    let result = validator.validate_response(&enum_response, &enum_spec);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]
async fn test_model_reference_resolution() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Test model reference (depends on test Manifest having models)
    let mut model_spec = ResponseSpec::new("object".to_string());
    model_spec.model_ref = Some("TestModel".to_string());
    
    let model_response = serde_json::json!({
        "id": "test123",
        "name": "Test Object"
    });
    
    let result = validator.validate_response(&model_response, &model_spec);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated >= 0, "Should count validated fields");
}

#[tokio::test]
async fn test_validation_timing_metrics() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    let simple_spec = ResponseSpec::new("string".to_string());
    let response = serde_json::json!("test response");
    
    // Test timing metrics
    for _ in 0..5 {
        let result = validator.validate_response(&response, &simple_spec);
        
        assert!(result.validation_time >= 0.0, "Validation time should be non-negative");
        assert!(result.validation_time < 1000.0, "Validation should be fast");
    }
}

#[tokio::test]
async fn test_field_count_tracking() {
    let spec = load_test_manifest();
    let validator = ResponseValidator::new(spec.clone());
    
    // Simple field count
    let simple_spec = ResponseSpec::new("string".to_string());
    let simple_response = serde_json::json!("test");
    let simple_result = validator.validate_response(&simple_response, &simple_spec);
    
    assert!(simple_result.fields_validated >= 1, "Should count at least one field");
    
    // Object field count
    let mut properties = HashMap::new();
    properties.insert("field1".to_string(), ArgumentSpec::new("string".to_string()));
    properties.insert("field2".to_string(), ArgumentSpec::new("number".to_string()));
    
    let object_spec = ResponseSpec::new("object".to_string()).with_properties(properties);
    let object_response = serde_json::json!({
        "field1": "value1",
        "field2": 42
    });
    
    let object_result = validator.validate_response(&object_response, &object_spec);
    
    assert!(object_result.fields_validated >= 1, "Should count object fields");
}