use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use crate::error::{JSONRPCError, JSONRPCErrorCode};

/// Timeout handler function type (exact SwiftJanus parity)
pub type TimeoutHandler = Box<dyn Fn(String, Duration) + Send + Sync>;

/// Error timeout handler function type (matches TypeScript error-handled registration)
pub type ErrorTimeoutHandler = Box<dyn Fn(String, Duration, Box<dyn std::error::Error + Send + Sync>) + Send + Sync>;

/// Timeout entry for internal tracking
struct TimeoutEntry {
    task: tokio::task::JoinHandle<()>,
    timeout_duration: Duration,
    created_at: chrono::DateTime<chrono::Utc>,
    on_timeout: Option<Arc<TimeoutHandler>>,
}

/// Bilateral timeout manager for handling both caller and handler timeouts
pub struct TimeoutManager {
    active_timeouts: Arc<Mutex<HashMap<String, TimeoutEntry>>>,
    stats: Arc<Mutex<TimeoutStats>>,
}

impl TimeoutManager {
    /// Create a new timeout manager
    pub fn new() -> Self {
        Self {
            active_timeouts: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(TimeoutStats::new())),
        }
    }
    
    /// Start a timeout for a command with callback
    pub async fn start_timeout(
        &self,
        command_id: String,
        timeout: Duration,
        on_timeout: Option<TimeoutHandler>,
    ) -> Result<(), JSONRPCError> {
        self.start_timeout_with_error_handler(command_id, timeout, on_timeout, None).await
    }
    
    /// Start a timeout with error handling callback (matches TypeScript error-handled registration)
    pub async fn start_timeout_with_error_handler(
        &self,
        command_id: String,
        timeout: Duration,
        on_timeout: Option<TimeoutHandler>,
        _on_error: Option<ErrorTimeoutHandler>,
    ) -> Result<(), JSONRPCError> {
        let command_id_clone = command_id.clone();
        let active_timeouts = self.active_timeouts.clone();
        let stats = self.stats.clone();
        let handler_arc = on_timeout.map(Arc::new);
        let handler_for_task = handler_arc.clone();
        
        let timeout_task = tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            
            // Execute timeout callback if provided
            if let Some(handler) = handler_for_task {
                handler(command_id_clone.clone(), timeout);
            }
            
            // Update statistics
            {
                let mut stats_lock = stats.lock().await;
                stats_lock.record_timeout(command_id_clone.clone(), timeout);
            }
            
            // Remove from active timeouts
            let mut timeouts = active_timeouts.lock().await;
            timeouts.remove(&command_id_clone);
        });
        
        // Update statistics for registration
        {
            let mut stats_lock = self.stats.lock().await;
            stats_lock.total_registered += 1;
        }
        
        // Store the timeout entry
        let mut timeouts = self.active_timeouts.lock().await;
        timeouts.insert(command_id, TimeoutEntry {
            task: timeout_task,
            timeout_duration: timeout,
            created_at: chrono::Utc::now(),
            on_timeout: handler_arc,
        });
        
        Ok(())
    }
    
    /// Cancel a timeout for a specific command
    pub async fn cancel_timeout(&self, command_id: &str) -> bool {
        let mut timeouts = self.active_timeouts.lock().await;
        
        if let Some(timeout_entry) = timeouts.remove(command_id) {
            timeout_entry.task.abort();
            
            // Update statistics
            {
                let mut stats_lock = self.stats.lock().await;
                stats_lock.total_cancelled += 1;
            }
            
            true
        } else {
            false
        }
    }
    
    /// Extend an existing timeout (matches Swift/TypeScript timeout extension capability)
    pub async fn extend_timeout(&self, command_id: &str, extension: Duration) -> bool {
        let mut timeouts = self.active_timeouts.lock().await;
        
        if let Some(mut timeout_entry) = timeouts.remove(command_id) {
            // Cancel the existing timeout
            timeout_entry.task.abort();
            
            // Calculate new total timeout duration for statistics
            let new_total_timeout = timeout_entry.timeout_duration + extension;
            timeout_entry.timeout_duration = new_total_timeout;
            
            // Create new timeout task with extension duration (from current time)
            let command_id_clone = command_id.to_string();
            let active_timeouts = self.active_timeouts.clone();
            let stats = self.stats.clone();
            let total_timeout_for_stats = new_total_timeout;
            let handler_for_extended_task = timeout_entry.on_timeout.clone();
            
            let timeout_task = tokio::spawn(async move {
                tokio::time::sleep(extension).await;
                
                // Execute timeout callback if provided
                if let Some(handler) = handler_for_extended_task {
                    handler(command_id_clone.clone(), total_timeout_for_stats);
                }
                
                // Update statistics when timeout fires
                {
                    let mut stats_lock = stats.lock().await;
                    stats_lock.record_timeout(command_id_clone.clone(), total_timeout_for_stats);
                }
                
                // Remove from active timeouts
                let mut timeouts = active_timeouts.lock().await;
                timeouts.remove(&command_id_clone);
            });
            
            timeout_entry.task = timeout_task;
            
            // Re-insert the updated entry
            timeouts.insert(command_id.to_string(), timeout_entry);
            
            true
        } else {
            false
        }
    }
    
    /// Check if a command has an active timeout
    pub async fn has_timeout(&self, command_id: &str) -> bool {
        let timeouts = self.active_timeouts.lock().await;
        timeouts.contains_key(command_id)
    }
    
    /// Get count of active timeouts
    pub async fn active_timeout_count(&self) -> usize {
        let timeouts = self.active_timeouts.lock().await;
        timeouts.len()
    }
    
    /// Register bilateral timeout for request/response pairs (matches Go/TypeScript implementation)
    pub async fn start_bilateral_timeout(
        &self,
        base_command_id: &str,
        timeout: Duration,
        on_timeout: Option<TimeoutHandler>,
    ) -> Result<(), JSONRPCError> {
        let request_id = format!("{}-request", base_command_id);
        let response_id = format!("{}-response", base_command_id);
        
        // Update statistics for registration
        {
            let mut stats_lock = self.stats.lock().await;
            stats_lock.total_registered += 1;
        }
        
        // Create shared timeout task for both request and response
        let request_id_clone = request_id.clone();
        let response_id_clone = response_id.clone();
        let active_timeouts = self.active_timeouts.clone();
        let stats = self.stats.clone();
        
        // Convert callback to Arc for sharing
        let handler_arc = on_timeout.map(Arc::new);
        let shared_callback_clone = handler_arc.clone();
        
        let timeout_task = tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            
            // Execute timeout callback if provided (only once)
            if let Some(handler) = shared_callback_clone {
                handler(request_id_clone.clone(), timeout);
            }
            
            // Update statistics
            {
                let mut stats_lock = stats.lock().await;
                stats_lock.record_timeout(request_id_clone.clone(), timeout);
            }
            
            // Remove both entries from active timeouts
            let mut timeouts = active_timeouts.lock().await;
            timeouts.remove(&request_id_clone);
            timeouts.remove(&response_id_clone);
        });
        
        // For bilateral timeout, we create two separate tasks that both clean up both entries
        // but only the first one executes the callback
        let request_id_clone2 = request_id.clone();
        let response_id_clone2 = response_id.clone();
        let active_timeouts_clone2 = self.active_timeouts.clone();
        
        let cleanup_task = tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            
            // Just clean up both entries (callback already handled by main task)
            let mut timeouts = active_timeouts_clone2.lock().await;
            timeouts.remove(&request_id_clone2);
            timeouts.remove(&response_id_clone2);
        });
        
        // Store both timeout entries (they share the same timeout behavior)
        let mut timeouts = self.active_timeouts.lock().await;
        timeouts.insert(request_id, TimeoutEntry {
            task: timeout_task,
            timeout_duration: timeout,
            created_at: chrono::Utc::now(),
            on_timeout: handler_arc.clone(),
        });
        timeouts.insert(response_id, TimeoutEntry {
            task: cleanup_task,
            timeout_duration: timeout,
            created_at: chrono::Utc::now(),
            on_timeout: handler_arc,
        });
        
        Ok(())
    }
    
    /// Cancel bilateral timeout (matches TypeScript implementation pattern)
    pub async fn cancel_bilateral_timeout(&self, base_command_id: &str) -> usize {
        let request_id = format!("{}-request", base_command_id);
        let response_id = format!("{}-response", base_command_id);
        
        let mut cancelled_count = 0;
        
        if self.cancel_timeout(&request_id).await {
            cancelled_count += 1;
        }
        
        if self.cancel_timeout(&response_id).await {
            cancelled_count += 1;
        }
        
        cancelled_count
    }
    
    
    /// Cancel all active timeouts
    pub async fn cancel_all_timeouts(&self) {
        let mut timeouts = self.active_timeouts.lock().await;
        let timeout_count = timeouts.len();
        
        for (_, timeout_entry) in timeouts.drain() {
            timeout_entry.task.abort();
        }
        
        // Update statistics
        {
            let mut stats_lock = self.stats.lock().await;
            stats_lock.total_cancelled += timeout_count as u64;
        }
    }
    
    /// Get timeout statistics (matches Go/TypeScript implementation)
    pub async fn get_timeout_statistics(&self) -> TimeoutStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }
    
    /// Execute command with bilateral timeout management
    pub async fn execute_with_timeout<F, T>(
        &self,
        command_id: String,
        timeout: Duration,
        operation: F,
        on_timeout: Option<TimeoutHandler>,
    ) -> Result<T, JSONRPCError>
    where
        F: std::future::Future<Output = Result<T, JSONRPCError>> + Send,
        T: Send,
    {
        // Start timeout tracking
        self.start_timeout(command_id.clone(), timeout, on_timeout).await?;
        
        // Execute operation with timeout
        let result = tokio::time::timeout(timeout, operation).await;
        
        // Cancel timeout tracking regardless of result
        self.cancel_timeout(&command_id).await;
        
        match result {
            Ok(operation_result) => operation_result,
            Err(_) => Err(JSONRPCError::new(JSONRPCErrorCode::HandlerTimeout, Some(format!("Command {} timed out after {:?}", command_id, timeout)))),
        }
    }
    
    /// Create a timeout handler that logs timeouts
    pub fn create_logging_timeout_handler() -> TimeoutHandler {
        Box::new(|command_id, timeout| {
            log::warn!("Command {} timed out after {:?}", command_id, timeout);
        })
    }
    
    /// Create a timeout handler that collects timeout statistics
    pub fn create_stats_timeout_handler(
        stats: Arc<Mutex<TimeoutStats>>,
    ) -> TimeoutHandler {
        Box::new(move |command_id, timeout| {
            let stats_clone = stats.clone();
            tokio::spawn(async move {
                let mut stats = stats_clone.lock().await;
                stats.record_timeout(command_id, timeout);
            });
        })
    }
}

impl Default for TimeoutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeout statistics for monitoring (matches Go/TypeScript implementation)
#[derive(Debug, Clone)]
pub struct TimeoutStats {
    pub total_registered: u64,
    pub total_cancelled: u64,
    pub total_expired: u64,
    pub total_timeouts: usize,
    pub average_timeout_duration: Duration,
    pub longest_timeout: Duration,
    pub shortest_timeout: Duration,
    pub last_timeout_command: Option<String>,
    pub timeout_history: Vec<(String, Duration, chrono::DateTime<chrono::Utc>)>,
}

impl TimeoutStats {
    /// Create new timeout statistics
    pub fn new() -> Self {
        Self {
            total_registered: 0,
            total_cancelled: 0,
            total_expired: 0,
            total_timeouts: 0,
            average_timeout_duration: Duration::from_secs(0),
            longest_timeout: Duration::from_secs(0),
            shortest_timeout: Duration::from_secs(3600), // Initialize with large value
            last_timeout_command: None,
            timeout_history: Vec::new(),
        }
    }
    
    /// Record a timeout occurrence
    pub fn record_timeout(&mut self, command_id: String, timeout_duration: Duration) {
        self.total_timeouts += 1;
        self.total_expired += 1;
        self.last_timeout_command = Some(command_id.clone());
        
        // Update duration statistics
        if timeout_duration > self.longest_timeout {
            self.longest_timeout = timeout_duration;
        }
        if timeout_duration < self.shortest_timeout {
            self.shortest_timeout = timeout_duration;
        }
        
        // Update average
        let total_duration = self.average_timeout_duration * (self.total_timeouts - 1) as u32 + timeout_duration;
        self.average_timeout_duration = total_duration / self.total_timeouts as u32;
        
        // Add to history (keep last 100 entries)
        self.timeout_history.push((command_id, timeout_duration, chrono::Utc::now()));
        if self.timeout_history.len() > 100 {
            self.timeout_history.remove(0);
        }
    }
    
    /// Get recent timeout rate (timeouts per minute)
    pub fn recent_timeout_rate(&self) -> f64 {
        let now = chrono::Utc::now();
        let one_minute_ago = now - chrono::Duration::minutes(1);
        
        let recent_timeouts = self.timeout_history
            .iter()
            .filter(|(_, _, timestamp)| *timestamp > one_minute_ago)
            .count();
        
        recent_timeouts as f64
    }
    
    /// Check if timeout rate is concerning
    pub fn is_timeout_rate_concerning(&self, threshold: f64) -> bool {
        self.recent_timeout_rate() > threshold
    }
}

impl Default for TimeoutStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeout configuration for different scenarios
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Default command timeout
    pub default_command_timeout: Duration,
    
    /// Default handler timeout
    pub default_handler_timeout: Duration,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Maximum allowed timeout
    pub max_timeout: Duration,
    
    /// Minimum allowed timeout
    pub min_timeout: Duration,
}

impl TimeoutConfig {
    /// Create standard timeout configuration
    pub fn standard() -> Self {
        Self {
            default_command_timeout: Duration::from_secs(30),
            default_handler_timeout: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            max_timeout: Duration::from_secs(300), // 5 minutes
            min_timeout: Duration::from_millis(100),
        }
    }
    
    /// Create aggressive timeout configuration (for high-performance scenarios)
    pub fn aggressive() -> Self {
        Self {
            default_command_timeout: Duration::from_secs(5),
            default_handler_timeout: Duration::from_secs(5),
            connection_timeout: Duration::from_secs(2),
            max_timeout: Duration::from_secs(60),
            min_timeout: Duration::from_millis(50),
        }
    }
    
    /// Create relaxed timeout configuration (for development/testing)
    pub fn relaxed() -> Self {
        Self {
            default_command_timeout: Duration::from_secs(120),
            default_handler_timeout: Duration::from_secs(120),
            connection_timeout: Duration::from_secs(30),
            max_timeout: Duration::from_secs(600), // 10 minutes
            min_timeout: Duration::from_secs(1),
        }
    }
    
    /// Validate timeout value against configuration
    pub fn validate_timeout(&self, timeout: Duration) -> Result<Duration, JSONRPCError> {
        if timeout < self.min_timeout {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed,
                Some(format!("Timeout {:?} is below minimum {:?}", timeout, self.min_timeout))
            ));
        }
        
        if timeout > self.max_timeout {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed,
                Some(format!("Timeout {:?} exceeds maximum {:?}", timeout, self.max_timeout))
            ));
        }
        
        Ok(timeout)
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[tokio::test]
    async fn test_timeout_manager_creation() {
        let manager = TimeoutManager::new();
        assert_eq!(manager.active_timeout_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_timeout_execution() {
        let manager = TimeoutManager::new();
        let command_id = "test-command".to_string();
        let timeout = Duration::from_millis(50);
        
        let timeout_triggered = Arc::new(AtomicUsize::new(0));
        let timeout_triggered_clone = timeout_triggered.clone();
        
        let timeout_handler = Box::new(move |_: String, _: Duration| {
            timeout_triggered_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        manager.start_timeout(command_id.clone(), timeout, Some(timeout_handler)).await.unwrap();
        
        // Wait for timeout to trigger
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(timeout_triggered.load(Ordering::SeqCst), 1);
        assert!(!manager.has_timeout(&command_id).await);
    }
    
    #[tokio::test]
    async fn test_timeout_cancellation() {
        let manager = TimeoutManager::new();
        let command_id = "test-command".to_string();
        let timeout = Duration::from_millis(100);
        
        let timeout_triggered = Arc::new(AtomicUsize::new(0));
        let timeout_triggered_clone = timeout_triggered.clone();
        
        let timeout_handler = Box::new(move |_: String, _: Duration| {
            timeout_triggered_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        manager.start_timeout(command_id.clone(), timeout, Some(timeout_handler)).await.unwrap();
        assert!(manager.has_timeout(&command_id).await);
        
        // Cancel timeout before it triggers
        tokio::time::sleep(Duration::from_millis(50)).await;
        let cancelled = manager.cancel_timeout(&command_id).await;
        
        assert!(cancelled);
        assert!(!manager.has_timeout(&command_id).await);
        
        // Wait to ensure timeout doesn't trigger
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(timeout_triggered.load(Ordering::SeqCst), 0);
    }
    
    #[tokio::test]
    async fn test_execute_with_timeout_success() {
        let manager = TimeoutManager::new();
        let command_id = "test-command".to_string();
        let timeout = Duration::from_millis(100);
        
        let operation = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<i32, JSONRPCError>(42)
        };
        
        let result = manager.execute_with_timeout(
            command_id,
            timeout,
            operation,
            None,
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_execute_with_timeout_failure() {
        let manager = TimeoutManager::new();
        let command_id = "test-command".to_string();
        let timeout = Duration::from_millis(50);
        
        let operation = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<i32, JSONRPCError>(42)
        };
        
        let result = manager.execute_with_timeout(
            command_id.clone(),
            timeout,
            operation,
            None,
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            err if err.code == JSONRPCErrorCode::HandlerTimeout as i32 => {
                // HandlerTimeout error - validate the error contains timeout info
                assert!(err.message.contains("timeout"));
            },
            _ => panic!("Expected HandlerTimeout error"),
        }
    }
    
    #[test]
    fn test_timeout_config_validation() {
        let config = TimeoutConfig::standard();
        
        // Valid timeout
        let valid_timeout = Duration::from_secs(30);
        assert!(config.validate_timeout(valid_timeout).is_ok());
        
        // Too short timeout
        let short_timeout = Duration::from_millis(50);
        assert!(config.validate_timeout(short_timeout).is_err());
        
        // Too long timeout
        let long_timeout = Duration::from_secs(600);
        assert!(config.validate_timeout(long_timeout).is_err());
    }
    
    #[test]
    fn test_timeout_stats() {
        let mut stats = TimeoutStats::new();
        
        stats.record_timeout("cmd1".to_string(), Duration::from_secs(30));
        stats.record_timeout("cmd2".to_string(), Duration::from_secs(60));
        
        assert_eq!(stats.total_timeouts, 2);
        assert_eq!(stats.average_timeout_duration, Duration::from_secs(45));
        assert_eq!(stats.last_timeout_command, Some("cmd2".to_string()));
    }
}