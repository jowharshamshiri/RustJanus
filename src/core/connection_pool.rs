use crate::core::UnixSocketClient;
use crate::config::UnixSockApiClientConfig;
use crate::error::UnixSockApiError;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Connection pool for managing Unix socket connections (exact Swift behavior)
pub struct ConnectionPool {
    socket_path: String,
    config: UnixSockApiClientConfig,
    available_connections: Arc<Mutex<VecDeque<UnixSocketClient>>>,
    active_connections: Arc<Mutex<usize>>,
    total_created: Arc<Mutex<usize>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(socket_path: String, config: UnixSockApiClientConfig) -> Self {
        Self {
            socket_path,
            config,
            available_connections: Arc::new(Mutex::new(VecDeque::new())),
            active_connections: Arc::new(Mutex::new(0)),
            total_created: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Borrow a connection from the pool (create new if needed)
    pub async fn borrow_connection(&self) -> Result<UnixSocketClient, UnixSockApiError> {
        // Try to get from available pool first
        {
            let mut available = self.available_connections.lock().await;
            if let Some(connection) = available.pop_front() {
                let mut active = self.active_connections.lock().await;
                *active += 1;
                return Ok(connection);
            }
        }
        
        // Check if we can create a new connection
        {
            let active_count = *self.active_connections.lock().await;
            if active_count >= self.config.max_concurrent_connections {
                return Err(UnixSockApiError::ResourceLimit(
                    format!("Maximum concurrent connections ({}) exceeded", 
                           self.config.max_concurrent_connections)
                ));
            }
        }
        
        // Create new connection
        let connection = UnixSocketClient::new(
            self.socket_path.clone(),
            self.config.clone()
        )?;
        
        // Update counters
        {
            let mut active = self.active_connections.lock().await;
            *active += 1;
        }
        {
            let mut total = self.total_created.lock().await;
            *total += 1;
        }
        
        Ok(connection)
    }
    
    /// Return a connection to the pool
    pub async fn return_connection(&self, connection: UnixSocketClient) {
        // Add to available pool
        {
            let mut available = self.available_connections.lock().await;
            available.push_back(connection);
        }
        
        // Decrement active count
        {
            let mut active = self.active_connections.lock().await;
            if *active > 0 {
                *active -= 1;
            }
        }
    }
    
    /// Get pool statistics
    pub async fn get_stats(&self) -> ConnectionPoolStats {
        let available_count = self.available_connections.lock().await.len();
        let active_count = *self.active_connections.lock().await;
        let total_created = *self.total_created.lock().await;
        
        ConnectionPoolStats {
            available_connections: available_count,
            active_connections: active_count,
            total_created,
            max_connections: self.config.max_concurrent_connections,
        }
    }
    
    /// Close all connections and clear the pool
    pub async fn close_all(&self) {
        // Clear available connections
        {
            let mut available = self.available_connections.lock().await;
            available.clear();
        }
        
        // Reset counters
        {
            let mut active = self.active_connections.lock().await;
            *active = 0;
        }
        {
            let mut total = self.total_created.lock().await;
            *total = 0;
        }
    }
    
    /// Trim pool to remove excess connections
    pub async fn trim_pool(&self, max_idle: usize) {
        let mut available = self.available_connections.lock().await;
        while available.len() > max_idle {
            available.pop_front();
        }
    }
    
    /// Check if pool is at capacity
    pub async fn is_at_capacity(&self) -> bool {
        let active_count = *self.active_connections.lock().await;
        active_count >= self.config.max_concurrent_connections
    }
    
    /// Get configuration
    pub fn config(&self) -> &UnixSockApiClientConfig {
        &self.config
    }
    
    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub available_connections: usize,
    pub active_connections: usize,
    pub total_created: usize,
    pub max_connections: usize,
}

impl ConnectionPoolStats {
    /// Calculate pool utilization as a percentage
    pub fn utilization_percent(&self) -> f64 {
        if self.max_connections == 0 {
            0.0
        } else {
            (self.active_connections as f64 / self.max_connections as f64) * 100.0
        }
    }
    
    /// Check if pool is under pressure
    pub fn is_under_pressure(&self) -> bool {
        self.utilization_percent() > 80.0
    }
    
    /// Check if pool is healthy
    pub fn is_healthy(&self) -> bool {
        self.active_connections <= self.max_connections && 
        self.available_connections < self.max_connections / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_pool_creation() {
        let socket_path = "/tmp/test.sock".to_string();
        let config = UnixSockApiClientConfig::default();
        
        let pool = ConnectionPool::new(socket_path, config);
        let stats = pool.get_stats().await;
        
        assert_eq!(stats.available_connections, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_created, 0);
    }
    
    #[tokio::test]
    async fn test_pool_capacity_limit() {
        let socket_path = "/tmp/test.sock".to_string();
        let config = UnixSockApiClientConfig {
            max_concurrent_connections: 2,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(socket_path, config);
        
        // This test would require a running server to actually borrow connections
        // For now, just test the capacity check
        assert!(!pool.is_at_capacity().await);
    }
    
    #[tokio::test]
    async fn test_pool_stats() {
        let socket_path = "/tmp/test.sock".to_string();
        let config = UnixSockApiClientConfig {
            max_concurrent_connections: 10,
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(socket_path, config);
        let stats = pool.get_stats().await;
        
        assert_eq!(stats.utilization_percent(), 0.0);
        assert!(!stats.is_under_pressure());
        assert!(stats.is_healthy());
    }
    
    #[test]
    fn test_stats_calculations() {
        let stats = ConnectionPoolStats {
            available_connections: 2,
            active_connections: 8,
            total_created: 10,
            max_connections: 10,
        };
        
        assert_eq!(stats.utilization_percent(), 80.0);
        assert!(!stats.is_under_pressure()); // Exactly 80%, not over
        
        let stats_pressure = ConnectionPoolStats {
            available_connections: 1,
            active_connections: 9,
            total_created: 10,
            max_connections: 10,
        };
        
        assert!(stats_pressure.is_under_pressure()); // 90% > 80%
    }
}