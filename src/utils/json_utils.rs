pub struct JsonUtils;

impl JsonUtils {
    pub fn validate_object(value: &serde_json::Value) -> bool {
        value.is_object()
    }
}