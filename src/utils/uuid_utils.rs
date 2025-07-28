pub struct UuidUtils;

impl UuidUtils {
    pub fn generate() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}