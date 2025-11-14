#!/bin/bash
# Helper script pour envoyer emails via msmtp

RECIPIENT="${1:-Markchavatte@gmail.com}"
SUBJECT="$2"
BODY="$3"

if [ -z "$SUBJECT" ] || [ -z "$BODY" ]; then
    echo "Usage: $0 [recipient] <subject> <body>"
    exit 1
fi

TIMESTAMP=$(date -Iseconds)

# Créer email
cat <<EOF | sudo msmtp "$RECIPIENT"
From: Symbion System <Markchavatte@gmail.com>
To: $RECIPIENT
Subject: $SUBJECT
Content-Type: text/plain; charset=UTF-8

$BODY

---
Envoyé depuis Symbion Automation System
Timestamp: $TIMESTAMP
EOF

if [ $? -eq 0 ]; then
    echo "✅ Email envoyé à $RECIPIENT"
else
    echo "❌ Erreur envoi email"
    exit 1
fi
