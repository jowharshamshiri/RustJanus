use crate::core::SecurityValidator;
use crate::error::{JSONRPCError, JSONRPCErrorCode};
use crate::config::JanusClientConfig;
use tokio::net::UnixDatagram;

/// Low-level Unix domain datagram socket client (SOCK_DGRAM)
/// Connectionless implementation for cross-language compatibility
#[derive(Debug, Clone)]
pub struct CoreJanusClient {
    socket_path: String,
    config: JanusClientConfig,
}

impl CoreJanusClient {
    /// Create a new Unix datagram client
    pub fn new(socket_path: String, config: JanusClientConfig) -> Result<Self, JSONRPCError> {
        // Validate socket path for security
        SecurityValidator::validate_socket_path(&socket_path)?;
        
        // Validate configuration
        config.validate()
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(e)))?;
        
        Ok(Self {
            socket_path,
            config,
        })
    }
    
    /// Send a datagram and receive response (connectionless communication)
    pub async fn send_datagram(&self, message: &[u8], response_socket_path: &str) -> Result<Vec<u8>, JSONRPCError> {
        // Validate message size
        SecurityValidator::validate_message_size(message.len(), &self.config)?;
        
        // Validate UTF-8 data
        SecurityValidator::validate_utf8_data(message)?;
        
        // Create response socket for receiving replies
        let response_socket = UnixDatagram::bind(response_socket_path)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to bind response socket: {}", e))))?;
        
        // Set timeout for response
        let timeout = tokio::time::timeout(
            self.config.connection_timeout,
            self.send_and_receive(message, &response_socket)
        );
        
        let result = timeout.await
            .map_err(|_| JSONRPCError::new(
                JSONRPCErrorCode::HandlerTimeout,
                Some(format!("datagram_send_receive timeout after {:?}", self.config.connection_timeout))
            ))?;
        
        // Clean up response socket
        std::fs::remove_file(response_socket_path).ok(); // Best effort cleanup
        
        result
    }
    
    /// Internal method to send datagram and receive response
    async fn send_and_receive(&self, message: &[u8], response_socket: &UnixDatagram) -> Result<Vec<u8>, JSONRPCError> {
        // Create client socket for sending
        let client_socket = UnixDatagram::unbound()
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to create client socket: {}", e))))?;
        
        // Send datagram to server
        client_socket.send_to(message, &self.socket_path)
            .await
            .map_err(|e| {
                // Check for message too long error
                if e.to_string().contains("message too long") || e.to_string().contains("Message too long") {
                    JSONRPCError::new(JSONRPCErrorCode::ResourceLimitExceeded, Some(format!(
                        "payload too large for SOCK_DGRAM (size: {} bytes): Unix domain datagram sockets have system-imposed size limits, typically around 64KB. Consider reducing payload size or using chunked messages", 
                        message.len()
                    )))
                } else {
                    JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to send datagram: {}", e)))
                }
            })?;
        
        // Receive response
        let mut buffer = vec![0u8; self.config.max_message_size];
        let (len, _) = response_socket.recv_from(&mut buffer)
            .await
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to receive response: {}", e))))?;
        
        buffer.truncate(len);
        Ok(buffer)
    }
    
    /// Send datagram without expecting response (fire-and-forget)
    pub async fn send_datagram_no_response(&self, message: &[u8]) -> Result<(), JSONRPCError> {
        // Validate message size
        SecurityValidator::validate_message_size(message.len(), &self.config)?;
        
        // Validate UTF-8 data
        SecurityValidator::validate_utf8_data(message)?;
        
        // Create client socket for sending
        let client_socket = UnixDatagram::unbound()
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to create client socket: {}", e))))?;
        
        // Send datagram to server
        client_socket.send_to(message, &self.socket_path)
            .await
            .map_err(|e| {
                // Check for message too long error
                if e.to_string().contains("message too long") || e.to_string().contains("Message too long") {
                    JSONRPCError::new(JSONRPCErrorCode::ResourceLimitExceeded, Some(format!(
                        "payload too large for SOCK_DGRAM (size: {} bytes): Unix domain datagram sockets have system-imposed size limits, typically around 64KB. Consider reducing payload size or using chunked messages", 
                        message.len()
                    )))
                } else {
                    JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to send datagram: {}", e)))
                }
            })?;
        
        Ok(())
    }
    
    /// Test connectivity to server socket
    pub async fn test_connection(&self) -> Result<(), JSONRPCError> {
        // Try to create client socket and send test message
        let client_socket = UnixDatagram::unbound()
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to create test socket: {}", e))))?;
        
        let test_message = b"test";
        client_socket.send_to(test_message, &self.socket_path)
            .await
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Connection test failed: {}", e))))?;
        
        Ok(())
    }
    
    /// Generate unique response socket path
    pub fn generate_response_socket_path(&self) -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = std::process::id();
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        
        // Use only allowed characters for security validator
        format!("/tmp/rust_janus_client_{}_{}__{}.sock", pid, timestamp, counter)
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