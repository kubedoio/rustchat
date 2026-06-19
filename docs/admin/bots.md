# Bot Accounts Administration

RustChat supports standard integration Bot Accounts that allow scripts, automation tools, and external services to interact with the API. 

Unlike [AI Agents](./ai-agents.md), standard Bot Accounts do not run an LLM prompt loop or vector-based RAG checks. They are intended for programmatic access (e.g. CI/CD notifications, ChatOps, custom alerts, or database syncing).

## Creating a Bot Account

To create a standard bot account, you must have administrative permissions:

1. Log in to the **Admin Console** or click on **Integrations** in the user menu.
2. Select **Bot Accounts** from the menu.
3. Click **Add Bot Account**.
4. Configure the bot details:
   - **Username**: Must follow standard username conventions (alphanumeric lowercase plus hyphens or underscores).
   - **Display Name**: Friendly name displayed on messages sent by the bot.
   - **Description**: Summary of the bot's purpose.
5. Save the Bot Account.

> [!IMPORTANT]
> The generated API access token is only shown once at creation time. Ensure you copy the token and store it securely.

## Authenticating API Calls as a Bot

Once you have the bot's access token, you can authenticate API requests by passing the token in the `Authorization` header as a Bearer token.

### Example curl request

To send a message to a channel as a bot:

```bash
curl -X POST http://localhost:8080/api/v4/posts \
  -H "Authorization: Bearer BOT_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "channel_id": "CHANNEL_UUID",
    "message": "Hello from the deployment bot! 🚀"
  }'
```

## Bot Scopes & Permissions

Standard bots inherit roles and permissions appropriate for their owner, or standard bot access roles:
- Bots can join public channels and read/write messages there.
- Bots must be invited to private channels before they can read or write in them.
- Standard bots cannot access the main system Admin Console settings or modify server configurations.

## Troubleshooting

### "Token is invalid or expired"
- Ensure the header uses the exact format: `Authorization: Bearer <token>`.
- Verify the bot is active and has not been disabled in the Integrations console.
- If a token is compromised or lost, you can revoke the token and create a new one under the bot's detail view in the Integrations console.
