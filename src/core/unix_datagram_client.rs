use crate::core::SecurityValidator;
use crate::error::UnixSockApiError;
use crate::config::UnixSockApiClientConfig;
use tokio::net::UnixDatagram;
use std::time::Duration;
use std::path::Path;

/// Low-level Unix domain datagram socket client (SOCK_DGRAM)
/// Connectionless implementation for cross-language compatibility
#[derive(Debug)]
pub struct UnixDatagramClient {
    socket_path: String,
    config: UnixSockApiClientConfig,
}

impl UnixDatagramClient {
    /// Create a new Unix datagram client
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
    
    /// Send a datagram and receive response (connectionless communication)
    pub async fn send_datagram(&self, message: &[u8], response_socket_path: &str) -> Result<Vec<u8>, UnixSockApiError> {
        // Validate message size
        SecurityValidator::validate_message_size(message.len(), &self.config)?;
        
        // Validate UTF-8 data
        SecurityValidator::validate_utf8_data(message)?;
        
        // Create response socket for receiving replies
        let response_socket = UnixDatagram::bind(response_socket_path)
            .await
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to bind response socket: {}", e)))?;
        
        // Set timeout for response
        let timeout = tokio::time::timeout(
            self.config.connection_timeout,
            self.send_and_receive(message, &response_socket)
        );
        
        let result = timeout.await
            .map_err(|_| UnixSockApiError::CommandTimeout(
                "datagram_send_receive".to_string(), 
                self.config.connection_timeout
            ))?;
        
        // Clean up response socket
        std::fs::remove_file(response_socket_path).ok(); // Best effort cleanup
        
        result
    }
    
    /// Internal method to send datagram and receive response
    async fn send_and_receive(&self, message: &[u8], response_socket: &UnixDatagram) -> Result<Vec<u8>, UnixSockApiError> {
        // Create client socket for sending
        let client_socket = UnixDatagram::unbound()
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to create client socket: {}", e)))?;
        
        // Send datagram to server
        client_socket.send_to(message, &self.socket_path)
            .await
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to send datagram: {}", e)))?;
        
        // Receive response
        let mut buffer = vec![0u8; self.config.max_message_size];
        let (len, _) = response_socket.recv_from(&mut buffer)
            .await
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to receive response: {}", e)))?;
        
        buffer.truncate(len);
        Ok(buffer)
    }
    
    /// Send datagram without expecting response (fire-and-forget)
    pub async fn send_datagram_no_response(&self, message: &[u8]) -> Result<(), UnixSockApiError> {
        // Validate message size
        SecurityValidator::validate_message_size(message.len(), &self.config)?;
        
        // Validate UTF-8 data
        SecurityValidator::validate_utf8_data(message)?;
        
        // Create client socket for sending
        let client_socket = UnixDatagram::unbound()
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to create client socket: {}", e)))?;
        
        // Send datagram to server
        client_socket.send_to(message, &self.socket_path)
            .await
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to send datagram: {}", e)))?;
        
        Ok(())
    }
    
    /// Test connectivity to server socket
    pub async fn test_connection(&self) -> Result<(), UnixSockApiError> {
        // Try to create client socket and send test message
        let client_socket = UnixDatagram::unbound()
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to create test socket: {}", e)))?;
        
        let test_message = b"test";
        client_socket.send_to(test_message, &self.socket_path)
            .await
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Connection test failed: {}", e)))?;
        
        Ok(())
    }
    
    /// Generate unique response socket path
    pub fn generate_response_socket_path(&self) -> String {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = std::process::id();
        format!("/tmp/rust_datagram_client_{}_{}.sock", pid, timestamp)
    }
    
    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
    
    /// Get maximum message size
    pub fn max_message_size(&self) -> usize {
        self.config.max_message_size
    }
}