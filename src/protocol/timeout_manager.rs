use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use crate::error::UnixSockApiError;

/// Timeout handler function type (exact SwiftUnixSockAPI parity)
pub type TimeoutHandler = Box<dyn Fn(String, Duration) + Send + Sync>;

/// Bilateral timeout manager for handling both caller and handler timeouts
pub struct TimeoutManager {
    active_timeouts: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl TimeoutManager {
    /// Create a new timeout manager
    pub fn new() -> Self {
        Self {
            active_timeouts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Start a timeout for a command with callback
    pub async fn start_timeout(
        &self,
        command_id: String,
        timeout: Duration,
        on_timeout: Option<TimeoutHandler>,
    ) -> Result<(), UnixSockApiError> {
        let command_id_clone = command_id.clone();
        let active_timeouts = self.active_timeouts.clone();
        
        let timeout_task = tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            
            // Execute timeout callback if provided
            if let Some(handler) = on_timeout {
                handler(command_id_clone.clone(), timeout);
            }
            
            // Remove from active timeouts
            let mut timeouts = active_timeouts.lock().await;
            timeouts.remove(&command_id_clone);
        });
        
        // Store the timeout task
        let mut timeouts = self.active_timeouts.lock().await;
        timeouts.insert(command_id, timeout_task);
        
        Ok(())
    }
    
    /// Cancel a timeout for a specific command
    pub async fn cancel_timeout(&self, command_id: &str) -> bool {
        let mut timeouts = self.active_timeouts.lock().await;
        
        if let Some(timeout_task) = timeouts.remove(command_id) {
            timeout_task.abort();
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
    
    /// Cancel all active timeouts
    pub async fn cancel_all_timeouts(&self) {
        let mut timeouts = self.active_timeouts.lock().await;
        
        for (_, timeout_task) in timeouts.drain() {
            timeout_task.abort();
        }
    }
    
    /// Execute command with bilateral timeout management
    pub async fn execute_with_timeout<F, T>(
        &self,
        command_id: String,
        timeout: Duration,
        operation: F,
        on_timeout: Option<TimeoutHandler>,
    ) -> Result<T, UnixSockApiError>
    where
        F: std::future::Future<Output = Result<T, UnixSockApiError>> + Send,
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
            Err(_) => Err(UnixSockApiError::CommandTimeout(command_id, timeout)),
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

/// Timeout statistics for monitoring
#[derive(Debug, Clone)]
pub struct TimeoutStats {
    pub total_timeouts: usize,
    pub average_timeout_duration: Duration,
    pub last_timeout_command: Option<String>,
    pub timeout_history: Vec<(String, Duration, chrono::DateTime<chrono::Utc>)>,
}

impl TimeoutStats {
    /// Create new timeout statistics
    pub fn new() -> Self {
        Self {
            total_timeouts: 0,
            average_timeout_duration: Duration::from_secs(0),
            last_timeout_command: None,
            timeout_history: Vec::new(),
        }
    }
    
    /// Record a timeout occurrence
    pub fn record_timeout(&mut self, command_id: String, timeout_duration: Duration) {
        self.total_timeouts += 1;
        self.last_timeout_command = Some(command_id.clone());
        
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
    pub fn validate_timeout(&self, timeout: Duration) -> Result<Duration, UnixSockApiError> {
        if timeout < self.min_timeout {
            return Err(UnixSockApiError::ValidationError(
                format!("Timeout {:?} is below minimum {:?}", timeout, self.min_timeout)
            ));
        }
        
        if timeout > self.max_timeout {
            return Err(UnixSockApiError::ValidationError(
                format!("Timeout {:?} exceeds maximum {:?}", timeout, self.max_timeout)
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
            Ok::<i32, UnixSockApiError>(42)
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
            Ok::<i32, UnixSockApiError>(42)
        };
        
        let result = manager.execute_with_timeout(
            command_id.clone(),
            timeout,
            operation,
            None,
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            UnixSockApiError::CommandTimeout(id, duration) => {
                assert_eq!(id, command_id);
                assert_eq!(duration, timeout);
            },
            _ => panic!("Expected CommandTimeout error"),
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