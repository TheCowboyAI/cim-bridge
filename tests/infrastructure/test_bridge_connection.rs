//! Infrastructure Layer 1.1: Bridge Connection Tests for cim-bridge
//! 
//! User Story: As a system integrator, I need to connect different services through a unified bridge
//!
//! Test Requirements:
//! - Verify NATS connection establishment
//! - Verify provider registration
//! - Verify service discovery
//! - Verify connection pooling
//!
//! Event Sequence:
//! 1. BridgeStarted { bridge_id, config }
//! 2. NATSConnected { connection_id, server_info }
//! 3. ProviderRegistered { provider_type, endpoint }
//! 4. ServiceDiscovered { service_name, capabilities }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Start Bridge]
//!     B --> C[BridgeStarted]
//!     C --> D[Connect NATS]
//!     D --> E[NATSConnected]
//!     E --> F[Register Provider]
//!     F --> G[ProviderRegistered]
//!     G --> H[Discover Services]
//!     H --> I[ServiceDiscovered]
//!     I --> J[Test Success]
//! ```

use std::collections::HashMap;
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Bridge configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub bridge_id: String,
    pub nats_url: String,
    pub providers: Vec<ProviderConfig>,
    pub max_connections: usize,
}

/// Provider configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub endpoint: String,
    pub capabilities: Vec<String>,
}

/// Provider types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderType {
    Ollama,
    OpenAI,
    Claude,
    Custom(String),
}

/// Bridge infrastructure events
#[derive(Debug, Clone, PartialEq)]
pub enum BridgeInfrastructureEvent {
    BridgeStarted { bridge_id: String, config: BridgeConfig },
    NATSConnected { connection_id: String, server_info: ServerInfo },
    ProviderRegistered { provider_type: ProviderType, endpoint: String },
    ServiceDiscovered { service_name: String, capabilities: Vec<String> },
    ConnectionFailed { reason: String },
}

/// Server information
#[derive(Debug, Clone, PartialEq)]
pub struct ServerInfo {
    pub server_id: String,
    pub version: String,
    pub protocol: u32,
}

/// Event stream validator for bridge testing
pub struct BridgeEventStreamValidator {
    expected_events: Vec<BridgeInfrastructureEvent>,
    captured_events: Vec<BridgeInfrastructureEvent>,
}

impl BridgeEventStreamValidator {
    pub fn new() -> Self {
        Self {
            expected_events: Vec::new(),
            captured_events: Vec::new(),
        }
    }

    pub fn expect_sequence(mut self, events: Vec<BridgeInfrastructureEvent>) -> Self {
        self.expected_events = events;
        self
    }

    pub fn capture_event(&mut self, event: BridgeInfrastructureEvent) {
        self.captured_events.push(event);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.captured_events.len() != self.expected_events.len() {
            return Err(format!(
                "Event count mismatch: expected {}, got {}",
                self.expected_events.len(),
                self.captured_events.len()
            ));
        }

        for (i, (expected, actual)) in self.expected_events.iter()
            .zip(self.captured_events.iter())
            .enumerate()
        {
            if expected != actual {
                return Err(format!(
                    "Event mismatch at position {}: expected {:?}, got {:?}",
                    i, expected, actual
                ));
            }
        }

        Ok(())
    }
}

/// Mock bridge manager
pub struct MockBridgeManager {
    config: BridgeConfig,
    connected: bool,
    providers: HashMap<ProviderType, ProviderConfig>,
}

impl MockBridgeManager {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            config,
            connected: false,
            providers: HashMap::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        // Simulate startup delay
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    pub async fn connect_nats(&mut self) -> Result<ServerInfo, String> {
        // Simulate connection delay
        tokio::time::sleep(Duration::from_millis(20)).await;
        
        if self.config.nats_url.is_empty() {
            return Err("NATS URL not configured".to_string());
        }

        self.connected = true;
        Ok(ServerInfo {
            server_id: "test-server-123".to_string(),
            version: "2.10.0".to_string(),
            protocol: 1,
        })
    }

    pub fn register_provider(&mut self, provider: ProviderConfig) -> Result<(), String> {
        if !self.connected {
            return Err("Bridge not connected".to_string());
        }

        self.providers.insert(provider.provider_type.clone(), provider);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn get_providers(&self) -> Vec<&ProviderConfig> {
        self.providers.values().collect()
    }
}

/// Service discovery manager
pub struct ServiceDiscoveryManager {
    services: HashMap<String, ServiceInfo>,
}

#[derive(Debug, Clone)]
struct ServiceInfo {
    name: String,
    capabilities: Vec<String>,
    endpoint: String,
}

impl ServiceDiscoveryManager {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    pub async fn discover_services(&mut self, provider: &ProviderConfig) -> Result<Vec<(String, Vec<String>)>, String> {
        // Simulate discovery delay
        tokio::time::sleep(Duration::from_millis(15)).await;

        let services = match &provider.provider_type {
            ProviderType::Ollama => vec![
                ("llama2".to_string(), vec!["chat".to_string(), "completion".to_string()]),
                ("codellama".to_string(), vec!["code".to_string(), "completion".to_string()]),
            ],
            ProviderType::OpenAI => vec![
                ("gpt-4".to_string(), vec!["chat".to_string(), "completion".to_string(), "vision".to_string()]),
                ("gpt-3.5-turbo".to_string(), vec!["chat".to_string(), "completion".to_string()]),
            ],
            ProviderType::Claude => vec![
                ("claude-3-opus".to_string(), vec!["chat".to_string(), "analysis".to_string()]),
                ("claude-3-sonnet".to_string(), vec!["chat".to_string(), "code".to_string()]),
            ],
            ProviderType::Custom(name) => vec![
                (format!("{}-service", name), vec!["custom".to_string()]),
            ],
        };

        for (name, caps) in &services {
            self.services.insert(name.clone(), ServiceInfo {
                name: name.clone(),
                capabilities: caps.clone(),
                endpoint: provider.endpoint.clone(),
            });
        }

        Ok(services)
    }

    pub fn get_service_count(&self) -> usize {
        self.services.len()
    }
}

/// Connection pool manager
pub struct ConnectionPoolManager {
    max_connections: usize,
    active_connections: Vec<ConnectionInfo>,
}

#[derive(Debug, Clone)]
struct ConnectionInfo {
    id: String,
    provider_type: ProviderType,
    created_at: std::time::Instant,
}

impl ConnectionPoolManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            active_connections: Vec::new(),
        }
    }

    pub fn acquire_connection(&mut self, provider_type: ProviderType) -> Result<String, String> {
        if self.active_connections.len() >= self.max_connections {
            return Err("Connection pool exhausted".to_string());
        }

        let conn_id = format!("conn_{}_{}", 
            self.active_connections.len(),
            chrono::Utc::now().timestamp_millis()
        );

        self.active_connections.push(ConnectionInfo {
            id: conn_id.clone(),
            provider_type,
            created_at: std::time::Instant::now(),
        });

        Ok(conn_id)
    }

    pub fn release_connection(&mut self, conn_id: &str) -> Result<(), String> {
        let pos = self.active_connections.iter()
            .position(|c| c.id == conn_id)
            .ok_or_else(|| "Connection not found".to_string())?;

        self.active_connections.remove(pos);
        Ok(())
    }

    pub fn get_active_count(&self) -> usize {
        self.active_connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_startup() {
        // Arrange
        let config = BridgeConfig {
            bridge_id: "test-bridge-001".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            providers: vec![],
            max_connections: 10,
        };

        let mut validator = BridgeEventStreamValidator::new()
            .expect_sequence(vec![
                BridgeInfrastructureEvent::BridgeStarted {
                    bridge_id: config.bridge_id.clone(),
                    config: config.clone(),
                },
            ]);

        let mut bridge = MockBridgeManager::new(config.clone());

        // Act
        bridge.start().await.unwrap();
        
        validator.capture_event(BridgeInfrastructureEvent::BridgeStarted {
            bridge_id: config.bridge_id.clone(),
            config,
        });

        // Assert
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_nats_connection() {
        // Arrange
        let config = BridgeConfig {
            bridge_id: "test-bridge-002".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            providers: vec![],
            max_connections: 10,
        };

        let mut bridge = MockBridgeManager::new(config);
        bridge.start().await.unwrap();

        // Act
        let server_info = bridge.connect_nats().await.unwrap();

        // Assert
        assert!(bridge.is_connected());
        assert_eq!(server_info.version, "2.10.0");
        assert_eq!(server_info.protocol, 1);
    }

    #[tokio::test]
    async fn test_provider_registration() {
        // Arrange
        let config = BridgeConfig {
            bridge_id: "test-bridge-003".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            providers: vec![],
            max_connections: 10,
        };

        let mut bridge = MockBridgeManager::new(config);
        bridge.start().await.unwrap();
        bridge.connect_nats().await.unwrap();

        let provider = ProviderConfig {
            provider_type: ProviderType::Ollama,
            endpoint: "http://localhost:11434".to_string(),
            capabilities: vec!["chat".to_string(), "completion".to_string()],
        };

        // Act
        bridge.register_provider(provider.clone()).unwrap();

        // Assert
        let providers = bridge.get_providers();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider_type, ProviderType::Ollama);
    }

    #[tokio::test]
    async fn test_service_discovery() {
        // Arrange
        let provider = ProviderConfig {
            provider_type: ProviderType::OpenAI,
            endpoint: "https://api.openai.com".to_string(),
            capabilities: vec!["chat".to_string()],
        };

        let mut discovery = ServiceDiscoveryManager::new();

        // Act
        let services = discovery.discover_services(&provider).await.unwrap();

        // Assert
        assert_eq!(services.len(), 2);
        assert!(services.iter().any(|(name, _)| name == "gpt-4"));
        assert!(services.iter().any(|(name, _)| name == "gpt-3.5-turbo"));
        assert_eq!(discovery.get_service_count(), 2);
    }

    #[tokio::test]
    async fn test_connection_pool() {
        // Arrange
        let mut pool = ConnectionPoolManager::new(3);

        // Act
        let conn1 = pool.acquire_connection(ProviderType::Ollama).unwrap();
        let _conn2 = pool.acquire_connection(ProviderType::OpenAI).unwrap();
        let _conn3 = pool.acquire_connection(ProviderType::Claude).unwrap();

        // Assert
        assert_eq!(pool.get_active_count(), 3);

        // Try to acquire one more (should fail)
        let result = pool.acquire_connection(ProviderType::Ollama);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exhausted"));

        // Release one and try again
        pool.release_connection(&conn1).unwrap();
        assert_eq!(pool.get_active_count(), 2);

        let _conn4 = pool.acquire_connection(ProviderType::Ollama).unwrap();
        assert_eq!(pool.get_active_count(), 3);
    }

    #[tokio::test]
    async fn test_empty_nats_url_failure() {
        // Arrange
        let config = BridgeConfig {
            bridge_id: "test-bridge-004".to_string(),
            nats_url: "".to_string(),
            providers: vec![],
            max_connections: 10,
        };

        let mut bridge = MockBridgeManager::new(config);

        // Act
        let result = bridge.connect_nats().await;

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("NATS URL not configured"));
        assert!(!bridge.is_connected());
    }

    #[tokio::test]
    async fn test_full_bridge_initialization_flow() {
        // Arrange
        let mut validator = BridgeEventStreamValidator::new();
        
        let provider_config = ProviderConfig {
            provider_type: ProviderType::Claude,
            endpoint: "https://api.anthropic.com".to_string(),
            capabilities: vec!["chat".to_string(), "analysis".to_string()],
        };

        let config = BridgeConfig {
            bridge_id: "test-bridge-005".to_string(),
            nats_url: "nats://localhost:4222".to_string(),
            providers: vec![provider_config.clone()],
            max_connections: 10,
        };

        let mut bridge = MockBridgeManager::new(config.clone());
        let mut discovery = ServiceDiscoveryManager::new();

        // Act
        // 1. Start bridge
        bridge.start().await.unwrap();
        validator.capture_event(BridgeInfrastructureEvent::BridgeStarted {
            bridge_id: config.bridge_id.clone(),
            config: config.clone(),
        });

        // 2. Connect NATS
        let server_info = bridge.connect_nats().await.unwrap();
        validator.capture_event(BridgeInfrastructureEvent::NATSConnected {
            connection_id: "test-conn-123".to_string(),
            server_info,
        });

        // 3. Register provider
        bridge.register_provider(provider_config.clone()).unwrap();
        validator.capture_event(BridgeInfrastructureEvent::ProviderRegistered {
            provider_type: provider_config.provider_type.clone(),
            endpoint: provider_config.endpoint.clone(),
        });

        // 4. Discover services
        let services = discovery.discover_services(&provider_config).await.unwrap();
        for (service_name, capabilities) in services {
            validator.capture_event(BridgeInfrastructureEvent::ServiceDiscovered {
                service_name,
                capabilities,
            });
        }

        // Assert
        assert!(bridge.is_connected());
        assert_eq!(bridge.get_providers().len(), 1);
        assert_eq!(discovery.get_service_count(), 2);
        assert_eq!(validator.captured_events.len(), 5); // 1 start + 1 connect + 1 register + 2 services
    }
} 