use std::time::Duration;

/// Configuration for UnixSockApiClient (exact SwiftUnixSockAPI parity)
#[derive(Debug, Clone)]
pub struct UnixSockApiClientConfig {
    /// Maximum number of concurrent connections (Default: 100)
    pub max_concurrent_connections: usize,
    
    /// Maximum message size in bytes (Default: 10MB)
    pub max_message_size: usize,
    
    /// Connection timeout duration (Default: 30.0s)
    pub connection_timeout: Duration,
    
    /// Maximum number of pending commands (Default: 1000)
    pub max_pending_commands: usize,
    
    /// Maximum number of command handlers (Default: 500)
    pub max_command_handlers: usize,
    
    /// Enable resource monitoring (Default: true)
    pub enable_resource_monitoring: bool,
    
    /// Maximum channel name length (Default: 256)
    pub max_channel_name_length: usize,
    
    /// Maximum command name length (Default: 256)
    pub max_command_name_length: usize,
    
    /// Maximum args data size in bytes (Default: 5MB)
    pub max_args_data_size: usize,
}

impl Default for UnixSockApiClientConfig {
    fn default() -> Self {
        Self {
            max_concurrent_connections: 100,
            max_message_size: 10_000_000,  // 10MB
            connection_timeout: Duration::from_secs(30),
            max_pending_commands: 1000,
            max_command_handlers: 500,
            enable_resource_monitoring: true,
            max_channel_name_length: 256,
            max_command_name_length: 256,
            max_args_data_size: 5_000_000,  // 5MB
        }
    }
}

impl UnixSockApiClientConfig {
    /// Create a new configuration with all default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a configuration optimized for high performance
    pub fn high_performance() -> Self {
        Self {
            max_concurrent_connections: 500,
            max_message_size: 50_000_000,  // 50MB
            connection_timeout: Duration::from_secs(60),
            max_pending_commands: 5000,
            max_command_handlers: 1000,
            enable_resource_monitoring: true,
            max_channel_name_length: 512,
            max_command_name_length: 512,
            max_args_data_size: 25_000_000,  // 25MB
        }
    }
    
    /// Create a configuration optimized for security (restrictive limits)
    pub fn secure() -> Self {
        Self {
            max_concurrent_connections: 10,
            max_message_size: 1_000_000,  // 1MB
            connection_timeout: Duration::from_secs(10),
            max_pending_commands: 100,
            max_command_handlers: 50,
            enable_resource_monitoring: true,
            max_channel_name_length: 128,
            max_command_name_length: 128,
            max_args_data_size: 500_000,  // 500KB
        }
    }
    
    /// Validate the configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.max_concurrent_connections == 0 {
            return Err("max_concurrent_connections must be greater than 0".to_string());
        }
        
        if self.max_message_size == 0 {
            return Err("max_message_size must be greater than 0".to_string());
        }
        
        if self.connection_timeout.is_zero() {
            return Err("connection_timeout must be greater than 0".to_string());
        }
        
        if self.max_pending_commands == 0 {
            return Err("max_pending_commands must be greater than 0".to_string());
        }
        
        if self.max_command_handlers == 0 {
            return Err("max_command_handlers must be greater than 0".to_string());
        }
        
        if self.max_channel_name_length == 0 {
            return Err("max_channel_name_length must be greater than 0".to_string());
        }
        
        if self.max_command_name_length == 0 {
            return Err("max_command_name_length must be greater than 0".to_string());
        }
        
        if self.max_args_data_size == 0 {
            return Err("max_args_data_size must be greater than 0".to_string());
        }
        
        Ok(())
    }
}