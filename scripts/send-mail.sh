#!/bin/bash
# Helper script pour envoyer emails via msmtp
# Utilise la même approche que monitor-symbion.sh

RECIPIENT="${1:-Markchavatte@gmail.com}"
SUBJECT="$2"
BODY="$3"

if [ -z "$SUBJECT" ] || [ -z "$BODY" ]; then
    echo "Usage: $0 [recipient] <subject> <body>"
    echo "Example: $0 'me@example.com' 'Subject' 'Message body'"
    exit 1
fi

TIMESTAMP=$(date -Iseconds)

# Ajouter timestamp au body
FULL_BODY="$BODY

---
Envoyé depuis Symbion Automation System
Timestamp: $TIMESTAMP"

# Envoyer via msmtp (même méthode que monitor-symbion.sh)
if command -v msmtp &> /dev/null; then
    echo -e "Subject: $SUBJECT\n\n$FULL_BODY" | msmtp "$RECIPIENT"
    if [ $? -eq 0 ]; then
        echo "✅ Email envoyé à $RECIPIENT via msmtp"
    else
        echo "❌ Erreur envoi email via msmtp"
        exit 1
    fi
elif command -v mail &> /dev/null; then
    echo "$FULL_BODY" | mail -s "$SUBJECT" "$RECIPIENT"
    if [ $? -eq 0 ]; then
        echo "✅ Email envoyé à $RECIPIENT via mail"
    else
        echo "❌ Erreur envoi email via mail"
        exit 1
    fi
else
    echo "❌ Aucun client email trouvé (msmtp ou mail)"
    echo "   Installer msmtp: sudo apt install msmtp"
    exit 1
fi
