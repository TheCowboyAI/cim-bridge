//! Infrastructure Layer 1.2: Message Routing Tests for cim-bridge
//! 
//! User Story: As a message broker, I need to route messages between providers and consumers
//!
//! Test Requirements:
//! - Verify message routing rules
//! - Verify topic subscription
//! - Verify message transformation
//! - Verify error handling
//!
//! Event Sequence:
//! 1. RouteConfigured { route_id, source, destination }
//! 2. MessageReceived { message_id, source, payload }
//! 3. MessageTransformed { message_id, transformation }
//! 4. MessageRouted { message_id, destination, latency }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Configure Route]
//!     B --> C[RouteConfigured]
//!     C --> D[Receive Message]
//!     D --> E[MessageReceived]
//!     E --> F[Transform Message]
//!     F --> G[MessageTransformed]
//!     G --> H[Route Message]
//!     H --> I[MessageRouted]
//!     I --> J[Test Success]
//! ```

use std::collections::HashMap;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

/// Message routing events
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRoutingEvent {
    RouteConfigured { route_id: String, source: String, destination: String },
    MessageReceived { message_id: String, source: String, payload: JsonValue },
    MessageTransformed { message_id: String, transformation: String },
    MessageRouted { message_id: String, destination: String, latency_ms: u64 },
    RoutingFailed { message_id: String, error: String },
}

/// Routing rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub route_id: String,
    pub source_pattern: String,
    pub destination: String,
    pub transformation: Option<TransformationType>,
    pub filters: Vec<MessageFilter>,
}

/// Transformation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationType {
    JsonToJson { template: String },
    ExtractField { field_path: String },
    AddMetadata { metadata: HashMap<String, JsonValue> },
    Custom { name: String },
}

/// Message filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageFilter {
    ContentType { mime_type: String },
    FieldExists { field_path: String },
    FieldEquals { field_path: String, value: JsonValue },
    SizeLimit { max_bytes: usize },
}

/// Message router
pub struct MessageRouter {
    routes: HashMap<String, RoutingRule>,
    topic_subscriptions: HashMap<String, Vec<String>>, // topic -> route_ids
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            topic_subscriptions: HashMap::new(),
        }
    }

    pub fn configure_route(&mut self, rule: RoutingRule) -> Result<(), String> {
        // Validate route
        if rule.source_pattern.is_empty() || rule.destination.is_empty() {
            return Err("Invalid route configuration".to_string());
        }

        // Subscribe to source pattern
        self.topic_subscriptions
            .entry(rule.source_pattern.clone())
            .or_insert_with(Vec::new)
            .push(rule.route_id.clone());

        self.routes.insert(rule.route_id.clone(), rule);
        Ok(())
    }

    pub fn route_message(
        &self,
        _message_id: String,
        source: String,
        payload: JsonValue,
    ) -> Result<Vec<(String, u64)>, String> {
        let start = Instant::now();
        let mut results = Vec::new();

        // Find matching routes
        let matching_routes = self.find_matching_routes(&source);

        for route_id in matching_routes {
            let route = self.routes.get(&route_id).unwrap();

            // Apply filters
            if !self.apply_filters(&route.filters, &payload)? {
                continue;
            }

            // Apply transformation
            let _transformed = if let Some(transformation) = &route.transformation {
                self.apply_transformation(transformation, &payload)?
            } else {
                payload.clone()
            };

            // Route to destination
            let latency = start.elapsed().as_millis() as u64;
            results.push((route.destination.clone(), latency));
        }

        if results.is_empty() {
            Err("No matching routes found".to_string())
        } else {
            Ok(results)
        }
    }

    fn find_matching_routes(&self, source: &str) -> Vec<String> {
        let mut matches = Vec::new();

        for (pattern, route_ids) in &self.topic_subscriptions {
            if self.matches_pattern(source, pattern) {
                matches.extend(route_ids.clone());
            }
        }

        matches
    }

    fn matches_pattern(&self, source: &str, pattern: &str) -> bool {
        // Simple pattern matching (could be enhanced with wildcards)
        if pattern.ends_with("*") {
            source.starts_with(&pattern[..pattern.len() - 1])
        } else {
            source == pattern
        }
    }

    fn apply_filters(&self, filters: &[MessageFilter], payload: &JsonValue) -> Result<bool, String> {
        for filter in filters {
            match filter {
                MessageFilter::ContentType { mime_type } => {
                    if let Some(ct) = payload.get("content_type").and_then(|v| v.as_str()) {
                        if ct != mime_type {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                }
                MessageFilter::FieldExists { field_path } => {
                    if self.get_field_value(payload, field_path).is_none() {
                        return Ok(false);
                    }
                }
                MessageFilter::FieldEquals { field_path, value } => {
                    if let Some(field_value) = self.get_field_value(payload, field_path) {
                        if field_value != value {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                }
                MessageFilter::SizeLimit { max_bytes } => {
                    let size = serde_json::to_string(payload)
                        .map_err(|e| e.to_string())?
                        .len();
                    if size > *max_bytes {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    fn get_field_value<'a>(&self, payload: &'a JsonValue, field_path: &str) -> Option<&'a JsonValue> {
        let parts: Vec<&str> = field_path.split('.').collect();
        let mut current = payload;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    fn apply_transformation(
        &self,
        transformation: &TransformationType,
        payload: &JsonValue,
    ) -> Result<JsonValue, String> {
        match transformation {
            TransformationType::ExtractField { field_path } => {
                self.get_field_value(payload, field_path)
                    .cloned()
                    .ok_or_else(|| format!("Field {} not found", field_path))
            }
            TransformationType::AddMetadata { metadata } => {
                let mut result = payload.clone();
                if let Some(obj) = result.as_object_mut() {
                    for (key, value) in metadata {
                        obj.insert(key.clone(), value.clone());
                    }
                }
                Ok(result)
            }
            _ => Ok(payload.clone()), // Simplified for other types
        }
    }

    pub fn get_route_count(&self) -> usize {
        self.routes.len()
    }
}

/// Message transformer
pub struct MessageTransformer {
    transformations: HashMap<String, Box<dyn Fn(&JsonValue) -> Result<JsonValue, String> + Send + Sync>>,
}

impl MessageTransformer {
    pub fn new() -> Self {
        Self {
            transformations: HashMap::new(),
        }
    }

    pub fn register_transformation<F>(&mut self, name: String, transform: F)
    where
        F: Fn(&JsonValue) -> Result<JsonValue, String> + Send + Sync + 'static,
    {
        self.transformations.insert(name, Box::new(transform));
    }

    pub fn transform(&self, name: &str, payload: &JsonValue) -> Result<JsonValue, String> {
        self.transformations
            .get(name)
            .ok_or_else(|| format!("Transformation {} not found", name))
            .and_then(|f| f(payload))
    }
}

/// Topic manager
pub struct TopicManager {
    topics: HashMap<String, TopicInfo>,
}

#[derive(Debug, Clone)]
struct TopicInfo {
    name: String,
    subscribers: Vec<String>,
    message_count: u64,
}

impl TopicManager {
    pub fn new() -> Self {
        Self {
            topics: HashMap::new(),
        }
    }

    pub fn create_topic(&mut self, name: String) -> Result<(), String> {
        if self.topics.contains_key(&name) {
            return Err("Topic already exists".to_string());
        }

        self.topics.insert(name.clone(), TopicInfo {
            name,
            subscribers: Vec::new(),
            message_count: 0,
        });

        Ok(())
    }

    pub fn subscribe(&mut self, topic: &str, subscriber_id: String) -> Result<(), String> {
        let topic_info = self.topics.get_mut(topic)
            .ok_or_else(|| "Topic not found".to_string())?;

        if !topic_info.subscribers.contains(&subscriber_id) {
            topic_info.subscribers.push(subscriber_id);
        }

        Ok(())
    }

    pub fn publish(&mut self, topic: &str, _message: &JsonValue) -> Result<Vec<String>, String> {
        let topic_info = self.topics.get_mut(topic)
            .ok_or_else(|| "Topic not found".to_string())?;

        topic_info.message_count += 1;
        Ok(topic_info.subscribers.clone())
    }

    pub fn get_topic_count(&self) -> usize {
        self.topics.len()
    }

    pub fn get_message_count(&self, topic: &str) -> Option<u64> {
        self.topics.get(topic).map(|info| info.message_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_route_configuration() {
        // Arrange
        let mut router = MessageRouter::new();
        let rule = RoutingRule {
            route_id: "route-001".to_string(),
            source_pattern: "chat.request.*".to_string(),
            destination: "llm.provider".to_string(),
            transformation: None,
            filters: vec![],
        };

        // Act
        let result = router.configure_route(rule);

        // Assert
        assert!(result.is_ok());
        assert_eq!(router.get_route_count(), 1);
    }

    #[test]
    fn test_message_routing() {
        // Arrange
        let mut router = MessageRouter::new();
        let rule = RoutingRule {
            route_id: "route-002".to_string(),
            source_pattern: "chat.request".to_string(),
            destination: "llm.provider".to_string(),
            transformation: None,
            filters: vec![],
        };
        router.configure_route(rule).unwrap();

        let message = json!({
            "content": "Hello, world!",
            "user_id": "user123"
        });

        // Act
        let results = router.route_message(
            "msg-001".to_string(),
            "chat.request".to_string(),
            message,
        ).unwrap();

        // Assert
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "llm.provider");
    }

    #[test]
    fn test_pattern_matching() {
        // Arrange
        let mut router = MessageRouter::new();
        let rule = RoutingRule {
            route_id: "route-003".to_string(),
            source_pattern: "events.*".to_string(),
            destination: "event.processor".to_string(),
            transformation: None,
            filters: vec![],
        };
        router.configure_route(rule).unwrap();

        let message = json!({ "type": "user.created" });

        // Act
        let results = router.route_message(
            "msg-002".to_string(),
            "events.user".to_string(),
            message,
        ).unwrap();

        // Assert
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "event.processor");
    }

    #[test]
    fn test_message_filtering() {
        // Arrange
        let mut router = MessageRouter::new();
        let rule = RoutingRule {
            route_id: "route-004".to_string(),
            source_pattern: "data.input".to_string(),
            destination: "data.processor".to_string(),
            transformation: None,
            filters: vec![
                MessageFilter::FieldExists { field_path: "user_id".to_string() },
                MessageFilter::FieldEquals {
                    field_path: "status".to_string(),
                    value: json!("active"),
                },
            ],
        };
        router.configure_route(rule).unwrap();

        let valid_message = json!({
            "user_id": "user123",
            "status": "active",
            "data": "test"
        });

        let invalid_message = json!({
            "user_id": "user123",
            "status": "inactive",
            "data": "test"
        });

        // Act
        let valid_results = router.route_message(
            "msg-003".to_string(),
            "data.input".to_string(),
            valid_message,
        );

        let invalid_results = router.route_message(
            "msg-004".to_string(),
            "data.input".to_string(),
            invalid_message,
        );

        // Assert
        assert!(valid_results.is_ok());
        assert!(invalid_results.is_err());
    }

    #[test]
    fn test_message_transformation() {
        // Arrange
        let mut router = MessageRouter::new();
        let rule = RoutingRule {
            route_id: "route-005".to_string(),
            source_pattern: "raw.data".to_string(),
            destination: "processed.data".to_string(),
            transformation: Some(TransformationType::ExtractField {
                field_path: "payload.content".to_string(),
            }),
            filters: vec![],
        };
        router.configure_route(rule).unwrap();

        let message = json!({
            "metadata": { "timestamp": "2024-01-01" },
            "payload": {
                "content": "Important data",
                "extra": "Not needed"
            }
        });

        // Act
        let results = router.route_message(
            "msg-005".to_string(),
            "raw.data".to_string(),
            message,
        ).unwrap();

        // Assert
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_topic_management() {
        // Arrange
        let mut topic_mgr = TopicManager::new();

        // Act
        topic_mgr.create_topic("chat.requests".to_string()).unwrap();
        topic_mgr.subscribe("chat.requests", "subscriber-001".to_string()).unwrap();
        topic_mgr.subscribe("chat.requests", "subscriber-002".to_string()).unwrap();

        let message = json!({ "content": "Hello" });
        let subscribers = topic_mgr.publish("chat.requests", &message).unwrap();

        // Assert
        assert_eq!(topic_mgr.get_topic_count(), 1);
        assert_eq!(subscribers.len(), 2);
        assert_eq!(topic_mgr.get_message_count("chat.requests"), Some(1));
    }

    #[test]
    fn test_custom_transformation() {
        // Arrange
        let mut transformer = MessageTransformer::new();
        
        transformer.register_transformation(
            "uppercase".to_string(),
            |payload| {
                if let Some(text) = payload.get("text").and_then(|v| v.as_str()) {
                    Ok(json!({ "text": text.to_uppercase() }))
                } else {
                    Err("Text field not found".to_string())
                }
            },
        );

        let input = json!({ "text": "hello world" });

        // Act
        let result = transformer.transform("uppercase", &input).unwrap();

        // Assert
        assert_eq!(result, json!({ "text": "HELLO WORLD" }));
    }

    #[test]
    fn test_size_limit_filter() {
        // Arrange
        let mut router = MessageRouter::new();
        let rule = RoutingRule {
            route_id: "route-006".to_string(),
            source_pattern: "upload.data".to_string(),
            destination: "storage.service".to_string(),
            transformation: None,
            filters: vec![
                MessageFilter::SizeLimit { max_bytes: 100 },
            ],
        };
        router.configure_route(rule).unwrap();

        let small_message = json!({ "data": "small" });
        let large_message = json!({
            "data": "This is a very large message that exceeds the size limit for routing through this particular route"
        });

        // Act
        let small_result = router.route_message(
            "msg-006".to_string(),
            "upload.data".to_string(),
            small_message,
        );

        let large_result = router.route_message(
            "msg-007".to_string(),
            "upload.data".to_string(),
            large_message,
        );

        // Assert
        assert!(small_result.is_ok());
        assert!(large_result.is_err());
    }
} 