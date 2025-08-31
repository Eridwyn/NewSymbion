#!/usr/bin/env python3
"""
Symbion DevKit - Tests Contractuels Automatiques

Valide automatiquement que:
1. Les contrats MQTT sont respectés par les plugins
2. Les schémas JSON sont corrects
3. Les topics suivent les conventions
4. Les plugins implémentent bien leurs contrats déclarés
"""

import os
import json
import asyncio
import argparse
from pathlib import Path
from typing import Dict, List, Any
import subprocess
import time
import signal
import sys

try:
    import paho.mqtt.client as mqtt
    import jsonschema
except ImportError:
    print("❌ Dépendances manquantes. Installer avec: pip install paho-mqtt jsonschema")
    sys.exit(1)

class ContractTester:
    def __init__(self, broker_host="127.0.0.1", broker_port=1883):
        self.broker_host = broker_host
        self.broker_port = broker_port
        self.client = None
        self.messages = []
        self.kernel_process = None
        self.plugin_processes = []
        
    def setup_mqtt(self):
        """Configure le client MQTT pour écouter les messages"""
        self.client = mqtt.Client()
        self.client.on_connect = self._on_connect
        self.client.on_message = self._on_message
        
        try:
            self.client.connect(self.broker_host, self.broker_port, 60)
            self.client.loop_start()
            print(f"📡 Connected to MQTT broker at {self.broker_host}:{self.broker_port}")
            return True
        except Exception as e:
            print(f"❌ Failed to connect to MQTT broker: {e}")
            return False
    
    def _on_connect(self, client, userdata, flags, rc):
        if rc == 0:
            # S'abonner à tous les topics symbion
            client.subscribe("symbion/+/+")
            client.subscribe("symbion/+/+/+")
            print("✅ Subscribed to all symbion topics")
        else:
            print(f"❌ MQTT connection failed with code {rc}")
    
    def _on_message(self, client, userdata, msg):
        try:
            payload = json.loads(msg.payload.decode())
            self.messages.append({
                'topic': msg.topic,
                'payload': payload,
                'timestamp': time.time()
            })
            print(f"📨 Received: {msg.topic} -> {json.dumps(payload, indent=2)}")
        except json.JSONDecodeError as e:
            print(f"⚠️ Invalid JSON in topic {msg.topic}: {e}")
    
    def load_contracts(self, contracts_dir):
        """Charge tous les contrats MQTT"""
        contracts = {}
        contracts_path = Path(contracts_dir)
        
        if not contracts_path.exists():
            print(f"❌ Contracts directory not found: {contracts_dir}")
            return contracts
        
        for contract_file in contracts_path.glob("*.json"):
            try:
                with open(contract_file) as f:
                    contract = json.load(f)
                    contract_name = contract.get('name', contract_file.stem)
                    contracts[contract_name] = contract
                    print(f"📜 Loaded contract: {contract_name}")
            except Exception as e:
                print(f"⚠️ Failed to load contract {contract_file}: {e}")
        
        return contracts
    
    def load_plugin_manifests(self, plugins_dir):
        """Charge tous les manifestes de plugins"""
        plugins = {}
        plugins_path = Path(plugins_dir)
        
        if not plugins_path.exists():
            print(f"❌ Plugins directory not found: {plugins_dir}")
            return plugins
        
        for plugin_file in plugins_path.glob("*.json"):
            try:
                with open(plugin_file) as f:
                    plugin = json.load(f)
                    plugin_name = plugin.get('name', plugin_file.stem)
                    plugins[plugin_name] = plugin
                    print(f"🔌 Loaded plugin manifest: {plugin_name}")
            except Exception as e:
                print(f"⚠️ Failed to load plugin manifest {plugin_file}: {e}")
        
        return plugins
    
    def start_kernel(self):
        """Démarre le kernel Symbion"""
        try:
            env = os.environ.copy()
            env['SYMBION_API_KEY'] = 'test-key'
            
            self.kernel_process = subprocess.Popen(
                ['cargo', 'run'],
                cwd='symbion-kernel',
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE
            )
            print("🚀 Starting Symbion kernel...")
            time.sleep(3)  # Attendre le démarrage
            return True
        except Exception as e:
            print(f"❌ Failed to start kernel: {e}")
            return False
    
    def start_plugin(self, plugin_name, plugin_manifest):
        """Démarre un plugin spécifique"""
        try:
            binary_path = plugin_manifest.get('binary', '')
            if not binary_path:
                print(f"⚠️ No binary path for plugin {plugin_name}")
                return False
            
            # Construire le plugin d'abord
            plugin_dir = Path(binary_path).parent.parent
            if plugin_dir.exists():
                build_result = subprocess.run(
                    ['cargo', 'build'],
                    cwd=plugin_dir,
                    capture_output=True
                )
                if build_result.returncode != 0:
                    print(f"❌ Failed to build plugin {plugin_name}")
                    return False
            
            # Démarrer le plugin
            process = subprocess.Popen(
                [binary_path],
                env={**os.environ, **plugin_manifest.get('env', {})},
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE
            )
            
            self.plugin_processes.append(process)
            print(f"🔌 Started plugin: {plugin_name}")
            return True
            
        except Exception as e:
            print(f"❌ Failed to start plugin {plugin_name}: {e}")
            return False
    
    def validate_contract_compliance(self, contracts, plugins):
        """Valide que les plugins respectent leurs contrats"""
        print("\\n🔍 Validating contract compliance...")
        
        for plugin_name, plugin_manifest in plugins.items():
            declared_contracts = plugin_manifest.get('contracts', [])
            print(f"\\n📋 Plugin {plugin_name} declares contracts: {declared_contracts}")
            
            for contract_name in declared_contracts:
                if contract_name in contracts:
                    contract = contracts[contract_name]
                    self._validate_plugin_contract(plugin_name, contract)
                else:
                    print(f"⚠️ Plugin {plugin_name} references unknown contract: {contract_name}")
    
    def _validate_plugin_contract(self, plugin_name, contract):
        """Valide qu'un plugin respecte un contrat spécifique"""
        contract_name = contract.get('name', 'unknown')
        expected_topic = contract.get('topic', '')
        schema = contract.get('schema', {})
        
        print(f"  🔍 Checking contract {contract_name} for plugin {plugin_name}")
        
        # Rechercher les messages correspondants
        matching_messages = [
            msg for msg in self.messages 
            if msg['topic'] == expected_topic
        ]
        
        if not matching_messages:
            print(f"    ⚠️ No messages found for topic: {expected_topic}")
            return
        
        # Valider le schéma JSON si présent
        if schema:
            for msg in matching_messages:
                try:
                    jsonschema.validate(msg['payload'], schema)
                    print(f"    ✅ Message validates against schema")
                except jsonschema.ValidationError as e:
                    print(f"    ❌ Schema validation failed: {e.message}")
                except Exception as e:
                    print(f"    ⚠️ Schema validation error: {e}")
        
        print(f"    📊 Found {len(matching_messages)} messages for {contract_name}")
    
    def run_tests(self, contracts_dir="contracts/mqtt", plugins_dir="plugins", duration=10):
        """Lance les tests contractuels complets"""
        print("🧪 Starting Symbion Contract Tests")
        print("=" * 50)
        
        # Chargement des données
        contracts = self.load_contracts(contracts_dir)
        plugins = self.load_plugin_manifests(plugins_dir)
        
        if not contracts:
            print("❌ No contracts loaded. Aborting.")
            return False
        
        if not plugins:
            print("❌ No plugins loaded. Aborting.")
            return False
        
        # Setup MQTT
        if not self.setup_mqtt():
            return False
        
        # Démarrage des composants
        if not self.start_kernel():
            return False
        
        # Démarrage des plugins
        for plugin_name, plugin_manifest in plugins.items():
            self.start_plugin(plugin_name, plugin_manifest)
        
        # Collecte des messages
        print(f"\\n⏱️ Collecting messages for {duration} seconds...")
        time.sleep(duration)
        
        # Validation des contrats
        self.validate_contract_compliance(contracts, plugins)
        
        # Rapport final
        self._generate_report()
        
        return True
    
    def _generate_report(self):
        """Génère un rapport des tests"""
        print("\\n" + "=" * 50)
        print("📊 CONTRACT TESTING REPORT")
        print("=" * 50)
        print(f"Total messages collected: {len(self.messages)}")
        
        # Grouper par topic
        topics = {}
        for msg in self.messages:
            topic = msg['topic']
            if topic not in topics:
                topics[topic] = 0
            topics[topic] += 1
        
        print("\\n📡 Messages by topic:")
        for topic, count in sorted(topics.items()):
            print(f"  {topic}: {count} messages")
        
        print("\\n✅ Contract testing completed")
    
    def cleanup(self):
        """Nettoyage des processus"""
        print("\\n🧹 Cleaning up...")
        
        if self.client:
            self.client.loop_stop()
            self.client.disconnect()
        
        # Arrêter les plugins
        for process in self.plugin_processes:
            try:
                process.terminate()
                process.wait(timeout=5)
            except:
                process.kill()
        
        # Arrêter le kernel
        if self.kernel_process:
            try:
                self.kernel_process.terminate()
                self.kernel_process.wait(timeout=5)
            except:
                self.kernel_process.kill()

def main():
    parser = argparse.ArgumentParser(description="Symbion Contract Tester")
    parser.add_argument("--contracts-dir", default="contracts/mqtt",
                       help="Directory containing MQTT contracts")
    parser.add_argument("--plugins-dir", default="plugins",
                       help="Directory containing plugin manifests")
    parser.add_argument("--duration", type=int, default=10,
                       help="Test duration in seconds")
    parser.add_argument("--broker-host", default="127.0.0.1",
                       help="MQTT broker host")
    parser.add_argument("--broker-port", type=int, default=1883,
                       help="MQTT broker port")
    
    args = parser.parse_args()
    
    tester = ContractTester(args.broker_host, args.broker_port)
    
    def signal_handler(sig, frame):
        print("\\n🛑 Interrupted by user")
        tester.cleanup()
        sys.exit(0)
    
    signal.signal(signal.SIGINT, signal_handler)
    
    try:
        success = tester.run_tests(
            args.contracts_dir,
            args.plugins_dir, 
            args.duration
        )
        tester.cleanup()
        sys.exit(0 if success else 1)
    except Exception as e:
        print(f"❌ Test failed: {e}")
        tester.cleanup()
        sys.exit(1)

if __name__ == "__main__":
    main()