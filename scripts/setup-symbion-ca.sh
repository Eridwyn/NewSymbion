#!/bin/bash
# Script de création d'une CA Symbion personnelle
# Usage: ./scripts/setup-symbion-ca.sh

set -e

CERTS_DIR="certs"
CA_DIR="$CERTS_DIR/ca"

echo "🔐 Création de l'Autorité de Certification Symbion"
echo "=================================================="

# Créer les répertoires
mkdir -p "$CA_DIR"
cd "$CERTS_DIR"

# 1️⃣ Générer la clé privée de la CA (à garder SECRÈTE)
echo ""
echo "📝 Étape 1/4 : Génération clé privée CA..."
openssl genrsa -out ca/symbion-ca.key 4096
chmod 600 ca/symbion-ca.key
echo "✅ Clé CA créée : ca/symbion-ca.key (CONFIDENTIEL)"

# 2️⃣ Créer le certificat root CA (valide 10 ans)
echo ""
echo "📝 Étape 2/4 : Génération certificat root CA..."
cat > /tmp/symbion-ca.conf <<EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_ca
prompt = no

[req_distinguished_name]
C = FR
O = Symbion Personal CA
OU = Home Automation
CN = Symbion Root CA

[v3_ca]
basicConstraints = critical,CA:TRUE
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer
EOF

openssl req -x509 -new -nodes \
  -key ca/symbion-ca.key \
  -sha256 -days 3650 \
  -out ca/symbion-ca.crt \
  -config /tmp/symbion-ca.conf \
  -extensions v3_ca

echo "✅ Certificat CA créé : ca/symbion-ca.crt (à installer sur tes appareils)"

# 3️⃣ Générer clé privée pour le kernel
echo ""
echo "📝 Étape 3/4 : Génération clé privée kernel..."
openssl genrsa -out key.pem 2048
echo "✅ Clé kernel créée : key.pem"

# 4️⃣ Créer certificat kernel signé par la CA
echo ""
echo "📝 Étape 4/4 : Génération certificat kernel signé..."

# Détecter l'IP locale automatiquement
LOCAL_IP=$(ip -4 addr show | grep -oP '(?<=inet\s)\d+(\.\d+){3}' | grep -v '127.0.0.1' | head -1)
HOSTNAME=$(hostname)

cat > /tmp/symbion-kernel.conf <<EOF
[req]
default_bits = 2048
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = FR
O = Symbion
CN = symbion-kernel

[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = symbion-kernel
DNS.3 = symbion.local
DNS.4 = $HOSTNAME
IP.1 = 127.0.0.1
IP.2 = $LOCAL_IP
EOF

# Créer CSR (Certificate Signing Request)
openssl req -new \
  -key key.pem \
  -out /tmp/symbion-kernel.csr \
  -config /tmp/symbion-kernel.conf

# Signer le certificat avec la CA
openssl x509 -req \
  -in /tmp/symbion-kernel.csr \
  -CA ca/symbion-ca.crt \
  -CAkey ca/symbion-ca.key \
  -CAcreateserial \
  -out cert.pem \
  -days 365 \
  -sha256 \
  -extfile /tmp/symbion-kernel.conf \
  -extensions v3_req

echo "✅ Certificat kernel signé créé : cert.pem"

# Sauvegarder les anciens certificats
if [ -f cert.pem.backup ]; then
  mv cert.pem.backup cert.pem.backup.old
fi
if [ -f key.pem.backup ]; then
  mv key.pem.backup key.pem.backup.old
fi

echo ""
echo "🎉 CA Symbion créée avec succès !"
echo "================================"
echo ""
echo "📂 Fichiers générés :"
echo "  - ca/symbion-ca.crt       → Certificat root CA (à installer)"
echo "  - ca/symbion-ca.key       → Clé CA (GARDER SECRET)"
echo "  - cert.pem                → Certificat kernel (signé par CA)"
echo "  - key.pem                 → Clé privée kernel"
echo ""
echo "📱 Prochaines étapes :"
echo ""
echo "1️⃣  Installer le certificat CA sur tes appareils"
echo ""
echo "   🖥️  Linux/PC :"
echo "      sudo cp ca/symbion-ca.crt /usr/local/share/ca-certificates/"
echo "      sudo update-ca-certificates"
echo ""
echo "   📱 Android :"
echo "      Paramètres → Sécurité → Certificats → Installer depuis stockage"
echo "      Sélectionner : ca/symbion-ca.crt"
echo ""
echo "   🍎 iPhone/iPad :"
echo "      Envoyer ca/symbion-ca.crt par email ou AirDrop"
echo "      Ouvrir → Installer le profil"
echo "      Réglages → Général → Informations → Réglages des certificats"
echo "      → Activer 'Symbion Root CA'"
echo ""
echo "   🪟 Windows :"
echo "      Double-clic ca/symbion-ca.crt"
echo "      Installer → Autorités de certification racines de confiance"
echo ""
echo "2️⃣  Redémarrer le kernel (il utilisera automatiquement les nouveaux certificats)"
echo ""
echo "3️⃣  Accéder à https://symbion.local:8443 ou https://$LOCAL_IP:8443"
echo "    → Aucune alerte de sécurité ! ✅"
echo ""
