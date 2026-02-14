#!/bin/bash
# Freebox Authorization Script
# Run this once to register the app and get the app_token

set -e

FREEBOX_URL="${FREEBOX_URL:-http://mafreebox.freebox.fr}"
APP_ID="${APP_ID:-symbion.freebox}"
APP_NAME="${APP_NAME:-Symbion Freebox}"
APP_VERSION="${APP_VERSION:-1.0}"
DEVICE_NAME="${DEVICE_NAME:-$(hostname)}"

echo "=== Symbion Freebox Authorization ==="
echo ""
echo "Freebox URL: $FREEBOX_URL"
echo "App ID: $APP_ID"
echo "App Name: $APP_NAME"
echo ""

# Step 1: Request authorization
echo "Step 1: Requesting authorization..."
RESPONSE=$(curl -s -X POST "$FREEBOX_URL/api/v8/login/authorize/" \
  -H "Content-Type: application/json" \
  -d "{
    \"app_id\": \"$APP_ID\",
    \"app_name\": \"$APP_NAME\",
    \"app_version\": \"$APP_VERSION\",
    \"device_name\": \"$DEVICE_NAME\"
  }")

SUCCESS=$(echo "$RESPONSE" | jq -r '.success')
if [ "$SUCCESS" != "true" ]; then
    echo "ERROR: Authorization request failed"
    echo "$RESPONSE" | jq .
    exit 1
fi

APP_TOKEN=$(echo "$RESPONSE" | jq -r '.result.app_token')
TRACK_ID=$(echo "$RESPONSE" | jq -r '.result.track_id')

echo ""
echo "=== IMPORTANT ==="
echo "Please validate the authorization on your Freebox:"
echo "1. Go to your Freebox (Freebox OS web interface or physical device)"
echo "2. Accept the authorization request for '$APP_NAME'"
echo ""
echo "Waiting for authorization..."
echo ""

# Step 2: Poll for authorization status
for i in {1..60}; do
    TRACK_RESPONSE=$(curl -s "$FREEBOX_URL/api/v8/login/authorize/$TRACK_ID")
    STATUS=$(echo "$TRACK_RESPONSE" | jq -r '.result.status')

    case "$STATUS" in
        "granted")
            echo ""
            echo "=== SUCCESS ==="
            echo ""
            echo "Authorization granted!"
            echo ""
            echo "Add this to your freebox.toml configuration:"
            echo ""
            echo "[freebox]"
            echo "app_id = \"$APP_ID\""
            echo "app_token = \"$APP_TOKEN\""
            echo ""
            echo "Token saved to: ./freebox_token.txt"
            echo "$APP_TOKEN" > freebox_token.txt
            exit 0
            ;;
        "denied")
            echo ""
            echo "ERROR: Authorization was denied"
            exit 1
            ;;
        "timeout")
            echo ""
            echo "ERROR: Authorization timed out"
            exit 1
            ;;
        "pending")
            printf "."
            sleep 2
            ;;
        *)
            echo ""
            echo "Unknown status: $STATUS"
            ;;
    esac
done

echo ""
echo "ERROR: Timed out waiting for authorization"
exit 1
