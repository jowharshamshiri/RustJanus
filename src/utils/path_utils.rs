use std::path::Path;

pub struct PathUtils;

impl PathUtils {
    pub fn ensure_socket_dir(socket_path: &str) -> std::io::Result<()> {
        if let Some(parent) = Path::new(socket_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
    
    pub fn cleanup_socket_file(socket_path: &str) -> std::io::Result<()> {
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }
        Ok(())
    }
}