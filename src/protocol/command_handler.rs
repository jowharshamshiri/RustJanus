use crate::error::{JanusError, JSONRPCError, JSONRPCErrorCode};
use crate::protocol::message_types::JanusCommand;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// Result of a handler execution
#[derive(Debug)]
pub enum HandlerResult<T> {
    Success(T),
    Error(JSONRPCError),
}

impl<T> HandlerResult<T> {
    pub fn success(value: T) -> Self {
        HandlerResult::Success(value)
    }
    
    pub fn error(error: JSONRPCError) -> Self {
        HandlerResult::Error(error)
    }
    
    pub fn from_result(result: Result<T, Box<dyn std::error::Error + Send + Sync>>) -> Self {
        match result {
            Ok(value) => HandlerResult::Success(value),
            Err(err) => HandlerResult::Error(JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(err.to_string()))),
        }
    }
}

/// Enhanced command handler trait for direct value responses
#[async_trait]
pub trait CommandHandler: Send + Sync {
    type Output: Serialize + Send;
    
    async fn handle(&self, command: &JanusCommand) -> HandlerResult<Self::Output>;
}

/// Synchronous handler wrapper
pub struct SyncHandler<F, T>
where
    F: Fn(&JanusCommand) -> HandlerResult<T> + Send + Sync,
    T: Serialize + Send,
{
    handler: F,
}

impl<F, T> SyncHandler<F, T>
where
    F: Fn(&JanusCommand) -> HandlerResult<T> + Send + Sync,
    T: Serialize + Send,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

#[async_trait]
impl<F, T> CommandHandler for SyncHandler<F, T>
where
    F: Fn(&JanusCommand) -> HandlerResult<T> + Send + Sync,
    T: Serialize + Send,
{
    type Output = T;
    
    async fn handle(&self, command: &JanusCommand) -> HandlerResult<Self::Output> {
        (self.handler)(command)
    }
}

/// Asynchronous handler wrapper
pub struct AsyncHandler<T>
where
    T: Serialize + Send,
{
    handler: Box<dyn Fn(&JanusCommand) -> std::pin::Pin<Box<dyn std::future::Future<Output = HandlerResult<T>> + Send>> + Send + Sync>,
}

impl<T> AsyncHandler<T>
where
    T: Serialize + Send,
{
    pub fn new<F, Fut>(handler: F) -> Self
    where
        F: Fn(&JanusCommand) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = HandlerResult<T>> + Send + 'static,
    {
        Self {
            handler: Box::new(move |cmd| Box::pin(handler(cmd))),
        }
    }
}

#[async_trait]
impl<T> CommandHandler for AsyncHandler<T>
where
    T: Serialize + Send,
{
    type Output = T;
    
    async fn handle(&self, command: &JanusCommand) -> HandlerResult<Self::Output> {
        (self.handler)(command).await
    }
}

/// Direct value handler constructors for common types

// Boolean handler
pub fn bool_handler<F>(handler: F) -> SyncHandler<impl Fn(&JanusCommand) -> HandlerResult<bool> + Send + Sync, bool>
where
    F: Fn(&JanusCommand) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
{
    SyncHandler::new(move |cmd| HandlerResult::from_result(handler(cmd)))
}

// String handler
pub fn string_handler<F>(handler: F) -> SyncHandler<impl Fn(&JanusCommand) -> HandlerResult<String> + Send + Sync, String>
where
    F: Fn(&JanusCommand) -> Result<String, Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
{
    SyncHandler::new(move |cmd| HandlerResult::from_result(handler(cmd)))
}

// Integer handler
pub fn int_handler<F>(handler: F) -> SyncHandler<impl Fn(&JanusCommand) -> HandlerResult<i64> + Send + Sync, i64>
where
    F: Fn(&JanusCommand) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
{
    SyncHandler::new(move |cmd| HandlerResult::from_result(handler(cmd)))
}

// Float handler
pub fn float_handler<F>(handler: F) -> SyncHandler<impl Fn(&JanusCommand) -> HandlerResult<f64> + Send + Sync, f64>
where
    F: Fn(&JanusCommand) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
{
    SyncHandler::new(move |cmd| HandlerResult::from_result(handler(cmd)))
}

// Array handler
pub fn array_handler<F, T>(handler: F) -> SyncHandler<impl Fn(&JanusCommand) -> HandlerResult<Vec<T>> + Send + Sync, Vec<T>>
where
    F: Fn(&JanusCommand) -> Result<Vec<T>, Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
    T: Serialize + Send,
{
    SyncHandler::new(move |cmd| HandlerResult::from_result(handler(cmd)))
}

// Object handler
pub fn object_handler<F, T>(handler: F) -> SyncHandler<impl Fn(&JanusCommand) -> HandlerResult<T> + Send + Sync, T>
where
    F: Fn(&JanusCommand) -> Result<T, Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
    T: Serialize + Send,
{
    SyncHandler::new(move |cmd| HandlerResult::from_result(handler(cmd)))
}

// Async boolean handler
pub fn async_bool_handler<F, Fut>(handler: F) -> AsyncHandler<bool>
where
    F: Fn(&JanusCommand) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<bool, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
    let handler = Arc::new(handler);
    AsyncHandler::new(move |cmd| {
        let handler = Arc::clone(&handler);
        let cmd = cmd.clone();
        async move {
            HandlerResult::from_result(handler(&cmd).await)
        }
    })
}

// Async string handler
pub fn async_string_handler<F, Fut>(handler: F) -> AsyncHandler<String>
where
    F: Fn(&JanusCommand) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
    let handler = Arc::new(handler);
    AsyncHandler::new(move |cmd| {
        let handler = Arc::clone(&handler);
        let cmd = cmd.clone();
        async move {
            HandlerResult::from_result(handler(&cmd).await)
        }
    })
}

// Async custom handler
pub fn async_custom_handler<F, Fut, T>(handler: F) -> AsyncHandler<T>
where
    F: Fn(&JanusCommand) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    T: Serialize + Send + 'static,
{
    let handler = Arc::new(handler);
    AsyncHandler::new(move |cmd| {
        let handler = Arc::clone(&handler);
        let cmd = cmd.clone();
        async move {
            HandlerResult::from_result(handler(&cmd).await)
        }
    })
}

/// Type-erased handler for registry storage
#[async_trait]
pub trait BoxedHandler: Send + Sync {
    async fn handle_boxed(&self, command: &JanusCommand) -> Result<serde_json::Value, JSONRPCError>;
}

#[async_trait]
impl<H> BoxedHandler for H
where
    H: CommandHandler + Send + Sync,
{
    async fn handle_boxed(&self, command: &JanusCommand) -> Result<serde_json::Value, JSONRPCError> {
        match self.handle(command).await {
            HandlerResult::Success(value) => {
                serde_json::to_value(value)
                    .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(e.to_string())))
            }
            HandlerResult::Error(error) => Err(error),
        }
    }
}

/// Enhanced handler registry with type safety
pub struct HandlerRegistry {
    handlers: Arc<RwLock<HashMap<String, Box<dyn BoxedHandler>>>>,
    max_handlers: usize,
}

impl HandlerRegistry {
    pub fn new(max_handlers: usize) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            max_handlers,
        }
    }
    
    pub async fn register_handler<H>(&self, command: String, handler: H) -> Result<(), JanusError>
    where
        H: CommandHandler + 'static,
    {
        let mut handlers = self.handlers.write().await;
        
        if handlers.len() >= self.max_handlers {
            return Err(JanusError::ResourceLimit(
                format!("Maximum handlers ({}) exceeded", self.max_handlers)
            ));
        }
        
        handlers.insert(command, Box::new(handler));
        Ok(())
    }
    
    pub async fn unregister_handler(&self, command: &str) -> bool {
        let mut handlers = self.handlers.write().await;
        handlers.remove(command).is_some()
    }
    
    pub async fn execute_handler(&self, command: &str, cmd: &JanusCommand) -> Result<serde_json::Value, JSONRPCError> {
        let handlers = self.handlers.read().await;
        
        match handlers.get(command) {
            Some(handler) => handler.handle_boxed(cmd).await,
            None => Err(JSONRPCError::new(
                JSONRPCErrorCode::MethodNotFound,
                Some(format!("Command not found: {}", command))
            )),
        }
    }
    
    pub async fn has_handler(&self, command: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(command)
    }
    
    pub async fn handler_count(&self) -> usize {
        let handlers = self.handlers.read().await;
        handlers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::message_types::JanusCommand;
    
    #[tokio::test]
    async fn test_bool_handler() {
        let handler = bool_handler(|_cmd| Ok(true));
        let cmd = JanusCommand::default();
        
        match handler.handle(&cmd).await {
            HandlerResult::Success(value) => assert_eq!(value, true),
            HandlerResult::Error(_) => panic!("Expected success"),
        }
    }
    
    #[tokio::test]
    async fn test_string_handler() {
        let handler = string_handler(|_cmd| Ok("Hello, World!".to_string()));
        let cmd = JanusCommand::default();
        
        match handler.handle(&cmd).await {
            HandlerResult::Success(value) => assert_eq!(value, "Hello, World!"),
            HandlerResult::Error(_) => panic!("Expected success"),
        }
    }
    
    #[tokio::test]
    async fn test_enhanced_registry() {
        let registry = HandlerRegistry::new(10);
        
        let handler = bool_handler(|_cmd| Ok(true));
        registry.register_handler("test".to_string(), handler).await.unwrap();
        
        assert!(registry.has_handler("test").await);
        assert_eq!(registry.handler_count().await, 1);
        
        let cmd = JanusCommand::default();
        let result = registry.execute_handler("test", &cmd).await.unwrap();
        assert_eq!(result, serde_json::Value::Bool(true));
    }
}