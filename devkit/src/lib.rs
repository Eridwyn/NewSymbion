/*!
# Symbion DevKit - Stubs et Utilitaires pour Développement

Bibliothèque facilitant le développement de plugins Symbion avec:
- Stubs MQTT pour tests sans broker
- Mocks des ports de données
- Helpers pour contrats JSON
- Clients de développement simplifiés
*/

pub mod mqtt_stub;
pub mod contract_helpers;
pub mod test_utils;

pub use mqtt_stub::MockMqttClient;
pub use contract_helpers::{ContractLoader, EventBuilder};
pub use test_utils::TestHarness;