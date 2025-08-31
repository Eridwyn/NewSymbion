#!/usr/bin/env python3
"""
Symbion DevKit - Plugin Scaffolding Tool

Génère rapidement un nouveau plugin Symbion avec:
- Structure Cargo.toml standardisée
- Template main.rs avec gestion MQTT
- Manifest JSON de plugin
- Tests contractuels de base
"""

import os
import json
import argparse
from pathlib import Path
import shutil
import re

def load_contract(contract_path):
    """Charge un contrat JSON et extrait les métadonnées"""
    with open(contract_path) as f:
        contract = json.load(f)
    
    return {
        'name': contract.get('name', ''),
        'version': contract.get('version', 'v1'),
        'topic': contract.get('topic', ''),
        'type': contract.get('type', 'event'),
        'schema': contract.get('schema', {})
    }

def generate_struct_from_schema(schema, struct_name):
    """Génère une struct Rust à partir d'un schéma JSON (basique)"""
    if not schema or 'properties' not in schema:
        return f"""
#[derive(Serialize, Deserialize, Debug, Clone)]
struct {struct_name} {{
    // TODO: Définir les champs selon le contrat
    placeholder: String,
}}"""
    
    fields = []
    for field_name, field_def in schema['properties'].items():
        rust_type = json_type_to_rust(field_def.get('type', 'string'))
        fields.append(f"    {field_name}: {rust_type},")
    
    return f"""
#[derive(Serialize, Deserialize, Debug, Clone)]
struct {struct_name} {{
{chr(10).join(fields)}
}}"""

def json_type_to_rust(json_type):
    """Conversion basique des types JSON vers Rust"""
    mapping = {
        'string': 'String',
        'number': 'f64',
        'integer': 'i64',
        'boolean': 'bool',
        'array': 'Vec<String>',  # Simplifié
        'object': 'serde_json::Value'
    }
    return mapping.get(json_type, 'String')

def camel_case(text):
    """Convertit en CamelCase"""
    return ''.join(word.capitalize() for word in re.split(r'[_\-.]', text))

def snake_case(text):
    """Convertit en snake_case"""
    return re.sub(r'[_\-.]', '_', text.lower())

def scaffold_plugin(plugin_name, contracts, description, output_dir):
    """Génère un plugin complet"""
    
    # Chemins
    templates_dir = Path(__file__).parent / "templates" / "plugin"
    plugin_dir = Path(output_dir) / plugin_name
    contracts_dir = Path(__file__).parent.parent / "contracts" / "mqtt"
    
    print(f"🔧 Generating plugin '{plugin_name}'...")
    
    # Créer la structure
    plugin_dir.mkdir(parents=True, exist_ok=True)
    (plugin_dir / "src").mkdir(exist_ok=True)
    
    # Charger les contrats
    contract_data = []
    input_topics = []
    output_topics = []
    handlers = []
    structs = []
    
    for contract_name in contracts:
        contract_file = contracts_dir / f"{contract_name}.json"
        if contract_file.exists():
            contract = load_contract(contract_file)
            contract_data.append(contract)
            
            struct_name = camel_case(contract['name'])
            structs.append({
                'name': struct_name,
                'definition': generate_struct_from_schema(contract.get('schema', {}), struct_name)
            })
            
            if contract['type'] == 'command':
                input_topics.append({
                    'topic': contract['topic'],
                    'struct_name': struct_name,
                    'handler_name': snake_case(contract['name'])
                })
                handlers.append({
                    'handler_name': snake_case(contract['name']),
                    'struct_name': struct_name
                })
            elif contract['type'] == 'event':
                output_topics.append({
                    'topic': contract['topic'],
                    'struct_name': struct_name
                })
    
    # Variables de template
    template_vars = {
        'PLUGIN_NAME': plugin_name,
        'PLUGIN_ID': snake_case(plugin_name),
        'PLUGIN_DESCRIPTION': description,
        'CONTRACTS': [{'CONTRACT_NAME': c, 'STRUCT_NAME': camel_case(c)} for c in contracts],
        'INPUT_TOPICS': input_topics,
        'OUTPUT_TOPICS': output_topics,
        'HANDLERS': handlers,
        'STRUCTS': structs,
        'ENV_VARS': []
    }
    
    # Générer Cargo.toml
    with open(templates_dir / "Cargo.toml.template") as f:
        cargo_content = f.read().replace("{{PLUGIN_NAME}}", plugin_name)
    
    with open(plugin_dir / "Cargo.toml", "w") as f:
        f.write(cargo_content)
    
    # Générer main.rs (template basique)
    with open(templates_dir / "src/main.rs.template") as f:
        main_content = f.read()
    
    # Remplacements basiques (Handlebars serait mieux mais gardons Python pur)
    main_content = main_content.replace("{{PLUGIN_NAME}}", plugin_name)
    
    # Ajouter les structs
    structs_code = "\n".join([s['definition'] for s in structs])
    main_content = main_content.replace("// ===== Data Structures =====", f"// ===== Data Structures ====={structs_code}")
    
    with open(plugin_dir / "src" / "main.rs", "w") as f:
        f.write(main_content)
    
    # Générer plugin.json
    manifest = {
        "name": snake_case(plugin_name),
        "version": "0.1.0",
        "binary": f"../target/debug/{plugin_name}",
        "description": description,
        "contracts": contracts,
        "auto_start": True,
        "restart_on_failure": True,
        "startup_timeout_seconds": 15,
        "shutdown_timeout_seconds": 5,
        "depends_on": [],
        "start_priority": 50,
        "env": {}
    }
    
    plugins_dir = Path(__file__).parent.parent / "plugins"
    with open(plugins_dir / f"{snake_case(plugin_name)}.json", "w") as f:
        json.dump(manifest, f, indent=2)
    
    print(f"✅ Plugin '{plugin_name}' generated successfully!")
    print(f"📁 Source code: {plugin_dir}")
    print(f"📄 Manifest: {plugins_dir / f'{snake_case(plugin_name)}.json'}")
    print(f"🔧 To build: cd {plugin_dir} && cargo build")

def main():
    parser = argparse.ArgumentParser(description="Symbion DevKit - Plugin Scaffolding")
    parser.add_argument("plugin_name", help="Nom du plugin (ex: my-awesome-plugin)")
    parser.add_argument("--contracts", nargs="+", default=[], 
                       help="Contrats MQTT utilisés (ex: heartbeat@v2 wake@v1)")
    parser.add_argument("--description", default="Plugin généré automatiquement",
                       help="Description du plugin")
    parser.add_argument("--output", default=".", 
                       help="Répertoire de sortie (défaut: répertoire courant)")
    
    args = parser.parse_args()
    
    scaffold_plugin(args.plugin_name, args.contracts, args.description, args.output)

if __name__ == "__main__":
    main()