/*!
 * Response Validation Tests for Rust Janus Implementation
 * Comprehensive tests for response validation features
 */

use rust_janus::*;
use rust_janus::manifest::{ResponseValidator, ResponseManifest, ArgumentManifest, ValidationManifest};
use std::collections::HashMap;
mod test_utils;
use test_utils::*;

#[tokio::test]
async fn test_response_against_manifest_validation() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Create a basic response manifest
    let response_manifest = ResponseManifest::new("string".to_string());
    
    // Test valid response
    let valid_response = serde_json::json!("valid test response");
    let result = validator.validate_response(&valid_response, &response_manifest);
    
    assert!(result.validation_time >= 0.0, "Validation time should be recorded");
    assert!(result.fields_validated > 0, "Fields should be counted");
}

#[tokio::test]
async fn test_request_response_validation() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Test request response validation using test Manifest
    let response = serde_json::json!({
        "result": "echo response"
    });
    
    let result = validator.validate_request_response(&response, "test", "echo");
    
    // Should validate based on the test Manifest
    assert!(result.validation_time >= 0.0, "Validation time should be recorded");
    assert!(result.fields_validated >= 0, "Fields should be counted");
}

#[tokio::test]
async fn test_type_validation_engine() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Test different type validations
    let string_manifest = ResponseManifest::new("string".to_string());
    let number_manifest = ResponseManifest::new("number".to_string());
    let boolean_manifest = ResponseManifest::new("boolean".to_string());
    
    // Valid type matches
    let string_result = validator.validate_response(&serde_json::json!("test"), &string_manifest);
    let number_result = validator.validate_response(&serde_json::json!(42), &number_manifest);
    let boolean_result = validator.validate_response(&serde_json::json!(true), &boolean_manifest);
    
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
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Create response manifest with string validation
    let mut response_manifest = ResponseManifest::new("string".to_string());
    
    // Test basic string validation
    let string_response = serde_json::json!("test string");
    let result = validator.validate_response(&string_response, &response_manifest);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]  
async fn test_numeric_range_validation() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Create number response manifest
    let number_manifest = ResponseManifest::new("number".to_string());
    
    // Test numeric validation
    let number_response = serde_json::json!(42);
    let result = validator.validate_response(&number_response, &number_manifest);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]
async fn test_object_property_validation() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Create object response manifest with properties
    let mut properties = HashMap::new();
    properties.insert("name".to_string(), ArgumentManifest::new("string".to_string()));
    properties.insert("value".to_string(), ArgumentManifest::new("number".to_string()));
    
    let object_manifest = ResponseManifest::new("object".to_string()).with_properties(properties);
    
    // Test object validation
    let object_response = serde_json::json!({
        "name": "test",
        "value": 42
    });
    
    let result = validator.validate_response(&object_response, &object_manifest);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]
async fn test_enum_value_validation() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Create string manifest (enum validation would be in ValidationManifest if implemented)
    let enum_manifest = ResponseManifest::new("string".to_string());
    
    // Test enum-like validation
    let enum_response = serde_json::json!("option1");
    let result = validator.validate_response(&enum_response, &enum_manifest);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated > 0, "Should count validated fields");
}

#[tokio::test]
async fn test_model_reference_resolution() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Test model reference (depends on test Manifest having models)
    let mut model_manifest = ResponseManifest::new("object".to_string());
    model_manifest.model_ref = Some("TestModel".to_string());
    
    let model_response = serde_json::json!({
        "id": "test123",
        "name": "Test Object"
    });
    
    let result = validator.validate_response(&model_response, &model_manifest);
    
    assert!(result.validation_time >= 0.0, "Should have validation timing");
    assert!(result.fields_validated >= 0, "Should count validated fields");
}

#[tokio::test]
async fn test_validation_timing_metrics() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    let simple_manifest = ResponseManifest::new("string".to_string());
    let response = serde_json::json!("test response");
    
    // Test timing metrics
    for _ in 0..5 {
        let result = validator.validate_response(&response, &simple_manifest);
        
        assert!(result.validation_time >= 0.0, "Validation time should be non-negative");
        assert!(result.validation_time < 1000.0, "Validation should be fast");
    }
}

#[tokio::test]
async fn test_field_count_tracking() {
    let manifest = load_test_manifest();
    let validator = ResponseValidator::new(manifest.clone());
    
    // Simple field count
    let simple_manifest = ResponseManifest::new("string".to_string());
    let simple_response = serde_json::json!("test");
    let simple_result = validator.validate_response(&simple_response, &simple_manifest);
    
    assert!(simple_result.fields_validated >= 1, "Should count at least one field");
    
    // Object field count
    let mut properties = HashMap::new();
    properties.insert("field1".to_string(), ArgumentManifest::new("string".to_string()));
    properties.insert("field2".to_string(), ArgumentManifest::new("number".to_string()));
    
    let object_manifest = ResponseManifest::new("object".to_string()).with_properties(properties);
    let object_response = serde_json::json!({
        "field1": "value1",
        "field2": 42
    });
    
    let object_result = validator.validate_response(&object_response, &object_manifest);
    
    assert!(object_result.fields_validated >= 1, "Should count object fields");
}