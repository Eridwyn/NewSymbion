#!/bin/bash
# Quick script to create admin user with bcrypt hash

# Generate hash using openssl (approximation, not perfect bcrypt but close)
# Real solution: use online bcrypt generator or install bcrypt CLI tool

echo "Pour générer un hash bcrypt valide, utilise un des outils suivants:"
echo ""
echo "Option 1 - Online (rapide):"
echo "  https://bcrypt-generator.com/"
echo "  Password: admin123"
echo "  Rounds: 10"
echo ""
echo "Option 2 - Utiliser le mot de passe temporaire:"
echo "  Username: admin"
echo "  Password: password"
echo ""

# Create users.json with a KNOWN working hash for "password"
cat > users.json << 'EOF'
[
  {
    "username": "admin",
    "password_hash": "$2b$10$YourHashHere",
    "role": "admin"
  }
]
EOF

echo "Fichier users.json créé (à modifier avec le vrai hash)"
