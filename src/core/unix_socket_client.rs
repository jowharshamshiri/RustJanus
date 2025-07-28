use crate::core::{MessageFrame, SecurityValidator};
use crate::error::UnixSockApiError;
use crate::config::UnixSockApiClientConfig;
use tokio::net::UnixStream;
use std::time::Duration;

/// Low-level Unix domain socket client with async/await (exact Swift implementation)
#[derive(Debug)]
pub struct UnixSocketClient {
    socket_path: String,
    config: UnixSockApiClientConfig,
}

impl UnixSocketClient {
    /// Create a new Unix socket client
    pub fn new(socket_path: String, config: UnixSockApiClientConfig) -> Result<Self, UnixSockApiError> {
        // Validate socket path for security
        SecurityValidator::validate_socket_path(&socket_path)?;
        
        // Validate configuration
        config.validate()
            .map_err(|e| UnixSockApiError::ValidationError(e))?;
        
        Ok(Self {
            socket_path,
            config,
        })
    }
    
    /// Send a message and receive response (stateless connection)
    pub async fn send_message(&self, message: &[u8]) -> Result<Vec<u8>, UnixSockApiError> {
        // Validate message size
        SecurityValidator::validate_message_size(message.len(), &self.config)?;
        
        // Validate UTF-8 data
        SecurityValidator::validate_utf8_data(message)?;
        
        // Create ephemeral connection
        let mut stream = tokio::time::timeout(
            self.config.connection_timeout,
            UnixStream::connect(&self.socket_path)
        ).await
        .map_err(|_| UnixSockApiError::CommandTimeout(
            "connection".to_string(), 
            self.config.connection_timeout
        ))?
        .map_err(|e| UnixSockApiError::ConnectionError(e.to_string()))?;
        
        // Send framed message
        MessageFrame::write_frame(&mut stream, message).await?;
        
        // Read framed response with timeout
        let response = tokio::time::timeout(
            self.config.connection_timeout,
            MessageFrame::read_frame(&mut stream)
        ).await
        .map_err(|_| UnixSockApiError::CommandTimeout(
            "response".to_string(),
            self.config.connection_timeout
        ))??;
        
        // Validate response size
        SecurityValidator::validate_message_size(response.len(), &self.config)?;
        
        // Validate response UTF-8
        SecurityValidator::validate_utf8_data(&response)?;
        
        Ok(response)
    }
    
    /// Send a message without waiting for response (fire-and-forget)
    pub async fn send_message_no_response(&self, message: &[u8]) -> Result<(), UnixSockApiError> {
        // Validate message size
        SecurityValidator::validate_message_size(message.len(), &self.config)?;
        
        // Validate UTF-8 data
        SecurityValidator::validate_utf8_data(message)?;
        
        // Create ephemeral connection
        let mut stream = tokio::time::timeout(
            self.config.connection_timeout,
            UnixStream::connect(&self.socket_path)
        ).await
        .map_err(|_| UnixSockApiError::CommandTimeout(
            "connection".to_string(),
            self.config.connection_timeout
        ))?
        .map_err(|e| UnixSockApiError::ConnectionError(e.to_string()))?;
        
        // Send framed message
        MessageFrame::write_frame(&mut stream, message).await?;
        
        // Close connection immediately (fire-and-forget)
        drop(stream);
        
        Ok(())
    }
    
    /// Test connection availability
    pub async fn test_connection(&self) -> Result<(), UnixSockApiError> {
        let stream = tokio::time::timeout(
            Duration::from_secs(5), // Short timeout for connection test
            UnixStream::connect(&self.socket_path)
        ).await
        .map_err(|_| UnixSockApiError::CommandTimeout(
            "connection_test".to_string(),
            Duration::from_secs(5)
        ))?
        .map_err(|e| UnixSockApiError::ConnectionError(e.to_string()))?;
        
        drop(stream);
        Ok(())
    }
    
    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
    
    /// Get configuration
    pub fn config(&self) -> &UnixSockApiClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::path::PathBuf;
    
    fn create_test_socket_path() -> PathBuf {
        let temp_dir = tempdir().unwrap();
        temp_dir.path().join("test.sock")
    }
    
    #[test]
    fn test_client_creation() {
        let socket_path = "/tmp/test.sock".to_string();
        let config = UnixSockApiClientConfig::default();
        
        let client = UnixSocketClient::new(socket_path, config);
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_invalid_socket_path() {
        let invalid_path = "/etc/passwd".to_string(); // Not in allowed directories
        let config = UnixSockApiClientConfig::default();
        
        let client = UnixSocketClient::new(invalid_path, config);
        assert!(client.is_err());
        
        match client.unwrap_err() {
            UnixSockApiError::SecurityViolation(_) => {},
            _ => panic!("Expected SecurityViolation error"),
        }
    }
    
    #[test]
    fn test_path_traversal_protection() {
        let malicious_path = "/tmp/../etc/passwd".to_string();
        let config = UnixSockApiClientConfig::default();
        
        let client = UnixSocketClient::new(malicious_path, config);
        assert!(client.is_err());
        
        match client.unwrap_err() {
            UnixSockApiError::SecurityViolation(msg) => {
                println!("Actual message: {}", msg);
                assert!(msg.to_lowercase().contains("path traversal"));
            },
            other => panic!("Expected SecurityViolation error for path traversal, got: {:?}", other),
        }
    }
    
    #[tokio::test]
    async fn test_message_size_validation() {
        let socket_path = "/tmp/test.sock".to_string();
        let config = UnixSockApiClientConfig {
            max_message_size: 100, // Small limit for testing
            ..Default::default()
        };
        
        let client = UnixSocketClient::new(socket_path, config).unwrap();
        
        // Test message that exceeds limit
        let large_message = vec![b'x'; 101];
        let result = client.send_message(&large_message).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            UnixSockApiError::MessageTooLarge(size, limit) => {
                assert_eq!(size, 101);
                assert_eq!(limit, 100);
            },
            _ => panic!("Expected MessageTooLarge error"),
        }
    }
}