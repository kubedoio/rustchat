#!/bin/bash

# Configuration
ENV_FILE=".env"

# Check if .env exists
if [ ! -f "$ENV_FILE" ]; then
    echo "Error: $ENV_FILE not found."
    exit 1
fi

echo "Generating secure keys..."

# Generate Keys and strip newlines using tr -d '\n'
# 64 bytes for JWT
JWT_SECRET=$(openssl rand -base64 64 | tr -d '\n')
# 32 bytes for Encryption
ENCRYPTION_KEY=$(openssl rand -base64 32 | tr -d '\n')

echo "Updating $ENV_FILE..."

# Detect OS for sed compatibility
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    SED_CMD="sed -i ''"
else
    # Linux
    SED_CMD="sed -i"
fi

# Replace RUSTCHAT_JWT_SECRET
$SED_CMD "s|RUSTCHAT_JWT_SECRET=.*|RUSTCHAT_JWT_SECRET=$JWT_SECRET|g" "$ENV_FILE"

# Replace RUSTCHAT_ENCRYPTION_KEY
$SED_CMD "s|RUSTCHAT_ENCRYPTION_KEY=.*|RUSTCHAT_ENCRYPTION_KEY=$ENCRYPTION_KEY|g" "$ENV_FILE"

echo "Success! Keys have been rotated in $ENV_FILE."
echo "New JWT Secret length: $(echo -n "$JWT_SECRET" | wc -c) chars"
echo "New Encryption Key length: $(echo -n "$ENCRYPTION_KEY" | wc -c) chars"
