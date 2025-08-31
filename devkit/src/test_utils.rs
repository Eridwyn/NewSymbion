/*!
Test Harness pour plugins Symbion

Facilite l'écriture de tests pour plugins avec:
- Setup automatique des mocks MQTT
- Assertions sur les événements échangés
- Simulation d'environnement Symbion complet
*/

use crate::mqtt_stub::MockMqttClient;
use crate::contract_helpers::{ContractLoader, EventBuilder};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;
use anyhow::Result;

/// Harness de test complet pour plugins Symbion
pub struct TestHarness {
    pub mqtt_client: MockMqttClient,
    pub contract_loader: ContractLoader,
    expectations: Vec<Expectation>,
}

#[derive(Debug)]
struct Expectation {
    topic: String,
    expected_count: usize,
    timeout_ms: u64,
}

impl TestHarness {
    /// Crée un nouveau harness de test
    pub fn new() -> Self {
        env_logger::try_init().ok(); // Init logging pour tests
        
        Self {
            mqtt_client: MockMqttClient::new(),
            contract_loader: ContractLoader::new("contracts"),
            expectations: Vec::new(),
        }
    }

    /// Configuration avec chargement automatique des contrats
    pub async fn with_contracts(mut self) -> Result<Self> {
        self.contract_loader.load_all_contracts()?;
        log::info!("📚 Loaded contracts for testing");
        Ok(self)
    }

    /// Ajoute une expectation: on s'attend à recevoir N messages sur un topic
    pub fn expect_messages(&mut self, topic: &str, count: usize) -> &mut Self {
        self.expectations.push(Expectation {
            topic: topic.to_string(),
            expected_count: count,
            timeout_ms: 5000, // 5s par défaut
        });
        self
    }

    /// Simule l'envoi d'un message vers le plugin testé
    pub async fn send_event(&self, contract_name: &str, event_data: Value) -> Result<()> {
        if let Some(contract) = self.contract_loader.get_contract(contract_name) {
            let topic = contract.topic.clone();
            let payload = serde_json::to_vec(&event_data)?;
            self.mqtt_client.simulate_incoming(topic, payload).await?;
            log::info!("📨 Sent test event: {}", contract_name);
        } else {
            anyhow::bail!("Contract not found: {}", contract_name);
        }
        Ok(())
    }

    /// Simule l'envoi d'un événement heartbeat
    pub async fn send_heartbeat(&self, host_id: &str, cpu: f32, ram: f32, ip: &str) -> Result<()> {
        use crate::mqtt_stub::SymbionMessageBuilder;
        
        let payload = SymbionMessageBuilder::heartbeat_v2(host_id, cpu, ram, ip);
        let payload_bytes = serde_json::to_vec(&payload)?;
        
        self.mqtt_client.simulate_incoming("symbion/hosts/heartbeat@v2", payload_bytes).await?;
        log::info!("💓 Sent heartbeat for host: {}", host_id);
        Ok(())
    }

    /// Simule l'envoi d'une commande wake
    pub async fn send_wake_command(&self, host_id: &str, mac: &str, broadcast: &str) -> Result<()> {
        use crate::mqtt_stub::SymbionMessageBuilder;
        
        let payload = SymbionMessageBuilder::wake_v1(host_id, mac, broadcast);
        let payload_bytes = serde_json::to_vec(&payload)?;
        
        self.mqtt_client.simulate_incoming("symbion/hosts/wake@v1", payload_bytes).await?;
        log::info!("⚡ Sent wake command for host: {}", host_id);
        Ok(())
    }

    /// Simule l'envoi d'une commande notes
    pub async fn send_notes_command(&self, action: &str, data: Value) -> Result<()> {
        use crate::mqtt_stub::SymbionMessageBuilder;
        
        let payload = SymbionMessageBuilder::notes_command_v1(action, data);
        let payload_bytes = serde_json::to_vec(&payload)?;
        
        self.mqtt_client.simulate_incoming("symbion/notes/command@v1", payload_bytes).await?;
        log::info!("📝 Sent notes command: {}", action);
        Ok(())
    }

    /// Attend et vérifie qu'un message a été publié sur un topic
    pub async fn wait_for_message(&self, topic: &str, timeout_ms: u64) -> Result<Option<Value>> {
        let start = std::time::Instant::now();
        
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            if let Some(msg) = self.mqtt_client.get_last_json_message::<Value>(topic)? {
                log::info!("✅ Received expected message on {}", topic);
                return Ok(Some(msg));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        
        log::warn!("⏰ Timeout waiting for message on {}", topic);
        Ok(None)
    }

    /// Vérifie toutes les expectations configurées
    pub async fn verify_expectations(&self) -> Result<()> {
        log::info!("🔍 Verifying {} expectations...", self.expectations.len());
        
        for expectation in &self.expectations {
            let messages = self.mqtt_client.find_messages_by_topic(&expectation.topic);
            let actual_count = messages.len();
            
            if actual_count != expectation.expected_count {
                anyhow::bail!(
                    "Expectation failed for topic '{}': expected {} messages, got {}",
                    expectation.topic, expectation.expected_count, actual_count
                );
            }
            
            log::info!("✅ Topic '{}': {} messages as expected", 
                      expectation.topic, actual_count);
        }
        
        log::info!("🎉 All expectations verified successfully");
        Ok(())
    }

    /// Assert qu'un message spécifique a été publié
    pub fn assert_message_sent(&self, topic: &str, expected_payload: &Value) -> Result<()> {
        let messages = self.mqtt_client.find_messages_by_topic(topic);
        
        for msg in messages {
            let payload: Value = serde_json::from_slice(&msg.payload)?;
            if payload == *expected_payload {
                log::info!("✅ Found expected message on {}", topic);
                return Ok(());
            }
        }
        
        anyhow::bail!("Expected message not found on topic: {}", topic);
    }

    /// Assert qu'un champ spécifique existe dans le dernier message
    pub fn assert_field_exists(&self, topic: &str, field_path: &str) -> Result<()> {
        if let Some(msg) = self.mqtt_client.get_last_json_message::<Value>(topic)? {
            if self.get_nested_field(&msg, field_path).is_some() {
                log::info!("✅ Field '{}' exists in {}", field_path, topic);
                return Ok(());
            }
        }
        
        anyhow::bail!("Field '{}' not found in latest message on {}", field_path, topic);
    }

    /// Assert qu'un champ a une valeur spécifique
    pub fn assert_field_equals(&self, topic: &str, field_path: &str, expected: &Value) -> Result<()> {
        if let Some(msg) = self.mqtt_client.get_last_json_message::<Value>(topic)? {
            if let Some(actual) = self.get_nested_field(&msg, field_path) {
                if actual == expected {
                    log::info!("✅ Field '{}' = {:?} in {}", field_path, expected, topic);
                    return Ok(());
                } else {
                    anyhow::bail!("Field '{}' mismatch: expected {:?}, got {:?}", 
                                 field_path, expected, actual);
                }
            }
        }
        
        anyhow::bail!("Field '{}' not found for comparison in {}", field_path, topic);
    }

    fn get_nested_field<'a>(&self, value: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;
        
        for part in parts {
            match current {
                Value::Object(obj) => {
                    current = obj.get(part)?;
                }
                _ => return None,
            }
        }
        
        Some(current)
    }

    /// Stats sur les messages collectés
    pub fn get_stats(&self) -> TestStats {
        let messages = self.mqtt_client.get_published_messages();
        let mut topic_counts = HashMap::new();
        
        for msg in &messages {
            *topic_counts.entry(msg.topic.clone()).or_insert(0) += 1;
        }
        
        TestStats {
            total_messages: messages.len(),
            topic_counts,
            subscriptions: self.mqtt_client.get_subscriptions(),
        }
    }

    /// Reset le harness pour un nouveau test
    pub fn reset(&mut self) {
        self.mqtt_client.clear();
        self.expectations.clear();
        log::info!("🧹 Test harness reset");
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct TestStats {
    pub total_messages: usize,
    pub topic_counts: HashMap<String, usize>,
    pub subscriptions: Vec<String>,
}

impl TestStats {
    pub fn print(&self) {
        println!("📊 Test Statistics:");
        println!("  Total messages: {}", self.total_messages);
        println!("  Topics with messages:");
        for (topic, count) in &self.topic_counts {
            println!("    {}: {} messages", topic, count);
        }
        println!("  Subscriptions: {:?}", self.subscriptions);
    }
}

/// Macro pour créer facilement des tests de plugins
#[macro_export]
macro_rules! plugin_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() {
            use $crate::test_utils::TestHarness;
            
            let mut harness = TestHarness::new().with_contracts().await.unwrap();
            let test_fn: Box<dyn Fn(&mut TestHarness) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>>>>> = Box::new($body);
            
            match test_fn(&mut harness).await {
                Ok(_) => {
                    harness.get_stats().print();
                    println!("✅ Test '{}' passed", stringify!($name));
                }
                Err(e) => {
                    eprintln!("❌ Test '{}' failed: {}", stringify!($name), e);
                    panic!("Test failed: {}", e);
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_basic_functionality() {
        let mut harness = TestHarness::new();
        
        // Test expectation
        harness.expect_messages("test/topic", 1);
        
        // Simuler l'envoi d'un message
        let test_data = serde_json::json!({"test": "value"});
        harness.mqtt_client.publish("test/topic", rumqttc::QoS::AtLeastOnce, false, 
                                   serde_json::to_vec(&test_data).unwrap()).await.unwrap();
        
        // Vérifier l'expectation
        harness.verify_expectations().await.unwrap();
        
        // Test des assertions
        harness.assert_message_sent("test/topic", &test_data).unwrap();
        
        let stats = harness.get_stats();
        assert_eq!(stats.total_messages, 1);
    }

    // Test avec la macro
    plugin_test!(test_macro_functionality, |harness: &mut TestHarness| {
        Box::pin(async move {
            let test_data = serde_json::json!({"macro_test": true});
            harness.mqtt_client.publish("macro/test", rumqttc::QoS::AtLeastOnce, false,
                                       serde_json::to_vec(&test_data)?).await?;
            
            harness.assert_field_equals("macro/test", "macro_test", &serde_json::Value::Bool(true))?;
            Ok(())
        })
    });
}