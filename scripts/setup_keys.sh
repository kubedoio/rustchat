#!/bin/bash

ENV_FILE=".env"
EXAMPLE_FILE=".env.example"

# 1. Check if .env exists
if [ ! -f "$ENV_FILE" ]; then
    echo "⚠️  Warning: No $ENV_FILE file found."
    echo "Please create one from your example file first by running:"
    echo ""
    echo "  cp $EXAMPLE_FILE $ENV_FILE"
    echo ""
    exit 1
fi

echo "Generating secure keys..."

# 2. Generate Keys (stripping newlines to prevent errors)
JWT_SECRET=$(openssl rand -base64 64 | tr -d '\n')
ENCRYPTION_KEY=$(openssl rand -base64 32 | tr -d '\n')

echo "Updating $ENV_FILE..."

# 3. Apply changes (Explicitly separated to fix the .env'' bug)
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS: requires the space and empty quotes
    sed -i '' "s|RUSTCHAT_JWT_SECRET=.*|RUSTCHAT_JWT_SECRET=$JWT_SECRET|g" "$ENV_FILE"
    sed -i '' "s|RUSTCHAT_ENCRYPTION_KEY=.*|RUSTCHAT_ENCRYPTION_KEY=$ENCRYPTION_KEY|g" "$ENV_FILE"
else
    # Linux: standard syntax
    sed -i "s|RUSTCHAT_JWT_SECRET=.*|RUSTCHAT_JWT_SECRET=$JWT_SECRET|g" "$ENV_FILE"
    sed -i "s|RUSTCHAT_ENCRYPTION_KEY=.*|RUSTCHAT_ENCRYPTION_KEY=$ENCRYPTION_KEY|g" "$ENV_FILE"
fi

echo "✅ Success! Keys have been rotated in $ENV_FILE."
