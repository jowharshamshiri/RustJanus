use crate::error::{UnixSockApiError, SocketError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Command handler function type (exact SwiftUnixSockAPI parity)
pub type CommandHandler = Arc<dyn Fn(HashMap<String, serde_json::Value>) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

/// Command handler registry for managing registered handlers
pub struct CommandHandlerRegistry {
    handlers: Arc<RwLock<HashMap<String, CommandHandler>>>,
    max_handlers: usize,
}

impl CommandHandlerRegistry {
    /// Create a new command handler registry
    pub fn new(max_handlers: usize) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            max_handlers,
        }
    }
    
    /// Register a command handler
    pub async fn register_handler(
        &self,
        command_name: String,
        handler: CommandHandler,
    ) -> Result<(), UnixSockApiError> {
        let mut handlers = self.handlers.write().await;
        
        // Check handler limit
        if handlers.len() >= self.max_handlers {
            return Err(UnixSockApiError::ResourceLimit(
                format!("Maximum command handlers ({}) exceeded", self.max_handlers)
            ));
        }
        
        // Register the handler
        handlers.insert(command_name, handler);
        Ok(())
    }
    
    /// Unregister a command handler
    pub async fn unregister_handler(&self, command_name: &str) -> bool {
        let mut handlers = self.handlers.write().await;
        handlers.remove(command_name).is_some()
    }
    
    /// Check if a handler is registered for a command
    pub async fn has_handler(&self, command_name: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(command_name)
    }
    
    /// Execute a command with its registered handler
    pub async fn execute_command(
        &self,
        command_name: &str,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<serde_json::Value, SocketError> {
        let handlers = self.handlers.read().await;
        
        let handler = handlers.get(command_name)
            .ok_or_else(|| SocketError::CommandNotFound(command_name.to_string()))?;
        
        let args = args.unwrap_or_default();
        
        // Execute handler
        handler(args)
            .map_err(|e| SocketError::ProcessingError(e.to_string()))
    }
    
    /// Get list of registered command names
    pub async fn get_registered_commands(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }
    
    /// Get count of registered handlers
    pub async fn handler_count(&self) -> usize {
        let handlers = self.handlers.read().await;
        handlers.len()
    }
    
    /// Clear all registered handlers
    pub async fn clear_handlers(&self) {
        let mut handlers = self.handlers.write().await;
        handlers.clear();
    }
    
    /// Check if registry is at capacity
    pub async fn is_at_capacity(&self) -> bool {
        self.handler_count().await >= self.max_handlers
    }
}

/// Helper functions for creating common command handlers
impl CommandHandlerRegistry {
    /// Create a simple echo handler
    pub fn create_echo_handler() -> CommandHandler {
        Arc::new(|args| {
            Ok(serde_json::json!({
                "echo": args,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        })
    }
    
    /// Create a status handler
    pub fn create_status_handler() -> CommandHandler {
        Arc::new(|_args| {
            Ok(serde_json::json!({
                "status": "ok",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "uptime_seconds": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        })
    }
    
    /// Create a ping handler
    pub fn create_ping_handler() -> CommandHandler {
        Arc::new(|_args| {
            Ok(serde_json::json!({
                "pong": true,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        })
    }
    
    /// Create a validation handler that validates required arguments
    pub fn create_validation_handler(required_args: Vec<String>) -> CommandHandler {
        Arc::new(move |args| {
            // Check for required arguments
            for required_arg in &required_args {
                if !args.contains_key(required_arg) {
                    return Err(format!("Missing required argument: {}", required_arg).into());
                }
            }
            
            Ok(serde_json::json!({
                "validation": "passed",
                "args_count": args.len(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        })
    }
    
    /// Create a handler that processes text commands
    pub fn create_text_processor_handler<F>(processor: F) -> CommandHandler 
    where
        F: Fn(&str) -> Result<String, String> + Send + Sync + 'static,
    {
        Arc::new(move |args| {
            let text = args.get("text")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'text' argument")?;
            
            let processed = processor(text)
                .map_err(|e| format!("Processing error: {}", e))?;
            
            Ok(serde_json::json!({
                "original": text,
                "processed": processed,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        })
    }
    
    /// Create a handler that manages key-value storage
    pub fn create_storage_handler(
        storage: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    ) -> CommandHandler {
        Arc::new(move |args| {
            let action = args.get("action")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'action' argument")?;
            
            match action {
                "get" => {
                    let key = args.get("key")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing 'key' argument for get action")?;
                    
                    let storage_clone = storage.clone();
                    let storage_guard = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(storage_clone.read())
                    });
                    
                    let value = storage_guard.get(key).cloned();
                    
                    Ok(serde_json::json!({
                        "action": "get",
                        "key": key,
                        "value": value,
                        "found": value.is_some()
                    }))
                },
                "set" => {
                    let key = args.get("key")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing 'key' argument for set action")?;
                    
                    let value = args.get("value")
                        .ok_or("Missing 'value' argument for set action")?
                        .clone();
                    
                    let storage_clone = storage.clone();
                    let mut storage_guard = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(storage_clone.write())
                    });
                    
                    storage_guard.insert(key.to_string(), value.clone());
                    
                    Ok(serde_json::json!({
                        "action": "set",
                        "key": key,
                        "value": value,
                        "success": true
                    }))
                },
                "delete" => {
                    let key = args.get("key")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing 'key' argument for delete action")?;
                    
                    let storage_clone = storage.clone();
                    let mut storage_guard = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(storage_clone.write())
                    });
                    
                    let removed = storage_guard.remove(key);
                    
                    Ok(serde_json::json!({
                        "action": "delete",
                        "key": key,
                        "removed": removed,
                        "success": removed.is_some()
                    }))
                },
                "list" => {
                    let storage_clone = storage.clone();
                    let storage_guard = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(storage_clone.read())
                    });
                    
                    let keys: Vec<&String> = storage_guard.keys().collect();
                    
                    Ok(serde_json::json!({
                        "action": "list",
                        "keys": keys,
                        "count": keys.len()
                    }))
                },
                _ => Err(format!("Unknown action: {}", action).into())
            }
        })
    }
}

/// Command execution context for providing additional information to handlers
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub command_id: String,
    pub channel_id: String,
    pub command_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub timeout: Option<std::time::Duration>,
}

impl CommandContext {
    pub fn new(
        command_id: String,
        channel_id: String,
        command_name: String,
        timeout: Option<std::time::Duration>,
    ) -> Self {
        Self {
            command_id,
            channel_id,
            command_name,
            timestamp: chrono::Utc::now(),
            timeout,
        }
    }
}

/// Enhanced command handler that receives context
pub type ContextualCommandHandler = Arc<dyn Fn(CommandContext, HashMap<String, serde_json::Value>) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

/// Registry for contextual command handlers
pub struct ContextualCommandHandlerRegistry {
    handlers: Arc<RwLock<HashMap<String, ContextualCommandHandler>>>,
    max_handlers: usize,
}

impl ContextualCommandHandlerRegistry {
    pub fn new(max_handlers: usize) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            max_handlers,
        }
    }
    
    pub async fn register_handler(
        &self,
        command_name: String,
        handler: ContextualCommandHandler,
    ) -> Result<(), UnixSockApiError> {
        let mut handlers = self.handlers.write().await;
        
        if handlers.len() >= self.max_handlers {
            return Err(UnixSockApiError::ResourceLimit(
                format!("Maximum command handlers ({}) exceeded", self.max_handlers)
            ));
        }
        
        handlers.insert(command_name, handler);
        Ok(())
    }
    
    pub async fn execute_command(
        &self,
        context: CommandContext,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<serde_json::Value, SocketError> {
        let handlers = self.handlers.read().await;
        
        let handler = handlers.get(&context.command_name)
            .ok_or_else(|| SocketError::CommandNotFound(context.command_name.clone()))?;
        
        let args = args.unwrap_or_default();
        
        handler(context, args)
            .map_err(|e| SocketError::ProcessingError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_registry_creation() {
        let registry = CommandHandlerRegistry::new(10);
        assert_eq!(registry.handler_count().await, 0);
        assert!(!registry.is_at_capacity().await);
    }
    
    #[tokio::test]
    async fn test_handler_registration() {
        let registry = CommandHandlerRegistry::new(10);
        let handler = CommandHandlerRegistry::create_echo_handler();
        
        let result = registry.register_handler("echo".to_string(), handler).await;
        assert!(result.is_ok());
        assert!(registry.has_handler("echo").await);
        assert_eq!(registry.handler_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_handler_execution() {
        let registry = CommandHandlerRegistry::new(10);
        let handler = CommandHandlerRegistry::create_echo_handler();
        
        registry.register_handler("echo".to_string(), handler).await.unwrap();
        
        let mut args = HashMap::new();
        args.insert("message".to_string(), serde_json::Value::String("hello".to_string()));
        
        let result = registry.execute_command("echo", Some(args.clone())).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.get("echo").is_some());
    }
    
    #[tokio::test]
    async fn test_unknown_command() {
        let registry = CommandHandlerRegistry::new(10);
        
        let result = registry.execute_command("unknown", None).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            SocketError::CommandNotFound(cmd) => {
                assert_eq!(cmd, "unknown");
            },
            _ => panic!("Expected CommandNotFound error"),
        }
    }
    
    #[tokio::test]
    async fn test_handler_limit() {
        let registry = CommandHandlerRegistry::new(2);
        let handler = CommandHandlerRegistry::create_echo_handler();
        
        // Register up to limit
        assert!(registry.register_handler("cmd1".to_string(), handler.clone()).await.is_ok());
        assert!(registry.register_handler("cmd2".to_string(), handler.clone()).await.is_ok());
        assert!(registry.is_at_capacity().await);
        
        // Try to register beyond limit
        let result = registry.register_handler("cmd3".to_string(), handler).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            UnixSockApiError::ResourceLimit(_) => {},
            _ => panic!("Expected ResourceLimit error"),
        }
    }
    
    #[tokio::test]
    async fn test_handler_unregistration() {
        let registry = CommandHandlerRegistry::new(10);
        let handler = CommandHandlerRegistry::create_echo_handler();
        
        registry.register_handler("echo".to_string(), handler).await.unwrap();
        assert!(registry.has_handler("echo").await);
        
        let unregistered = registry.unregister_handler("echo").await;
        assert!(unregistered);
        assert!(!registry.has_handler("echo").await);
        
        let unregistered_again = registry.unregister_handler("echo").await;
        assert!(!unregistered_again);
    }
    
    #[tokio::test(flavor = "multi_thread")]
    async fn test_storage_handler() {
        let storage = Arc::new(RwLock::new(HashMap::new()));
        let handler = CommandHandlerRegistry::create_storage_handler(storage);
        
        // Test set operation
        let mut set_args = HashMap::new();
        set_args.insert("action".to_string(), serde_json::Value::String("set".to_string()));
        set_args.insert("key".to_string(), serde_json::Value::String("test_key".to_string()));
        set_args.insert("value".to_string(), serde_json::Value::String("test_value".to_string()));
        
        let result = handler(set_args).unwrap();
        assert_eq!(result["success"], true);
        
        // Test get operation
        let mut get_args = HashMap::new();
        get_args.insert("action".to_string(), serde_json::Value::String("get".to_string()));
        get_args.insert("key".to_string(), serde_json::Value::String("test_key".to_string()));
        
        let result = handler(get_args).unwrap();
        assert_eq!(result["found"], true);
        assert_eq!(result["value"], "test_value");
    }
    
    #[test]
    fn test_command_context() {
        let context = CommandContext::new(
            "test-id".to_string(),
            "test-channel".to_string(),
            "test-command".to_string(),
            Some(std::time::Duration::from_secs(30)),
        );
        
        assert_eq!(context.command_id, "test-id");
        assert_eq!(context.channel_id, "test-channel");
        assert_eq!(context.command_name, "test-command");
        assert!(context.timeout.is_some());
    }
}