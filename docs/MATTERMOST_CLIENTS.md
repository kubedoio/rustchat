# Connecting Mattermost Clients to Rustchat

Rustchat implements a subset of the Mattermost v4 API, allowing you to use official Mattermost clients (Desktop and Mobile) to connect to your Rustchat server.

## Compatibility

Rustchat currently supports the following features in Mattermost clients:
- Authentication (Email/Password)
- Real-time messaging (WebSocket)
- Channel / Team navigation
- Posting messages
- Thread replies (partial)

*Note: This is a work in progress. Some features like advanced search, extensive settings, or specific plugins may not be fully supported.*

### Native Web Frontend vs. Mattermost Clients

It is important to note that the **Rustchat Web Frontend** (included in the Docker setup) is a separate application from the Mattermost clients.
-   **Rustchat Web**: Uses the native Rustchat API (`/api/v1`). It **does not** require or use the Mattermost compatibility layer (`/api/v4`).
-   **Mattermost Clients**: Exclusively use the `/api/v4` compatibility layer.

Changes made to the Mattermost compatibility layer (e.g., API version strings, system pings) do not affect the native web frontend.

## Prerequisites

Ensure your Rustchat server is running. If you are using the provided Docker Compose setup:
- **Frontend/Proxy**: Running on port `8080` (default)
- **Backend**: Running on port `3000` (default)

## Ports

To allow clients to connect, you must ensure the following ports are open and accessible from the client device:

| Port | Service | Description |
|------|---------|-------------|
| **8080** | Nginx Proxy | **Recommended.** Handles both the Web UI and API requests (`/api/v4`). Use this port for your Server URL. |
| **3000** | Backend API | Direct access to the backend API. Can be used if you want to bypass the Nginx proxy, but using 8080 is preferred. |

## Connection Instructions

1.  **Download Client**: Download the official Mattermost app for your platform:
    -   [Mattermost Desktop App](https://mattermost.com/apps/) (Windows, macOS, Linux)
    -   [Mattermost Mobile App](https://mattermost.com/apps/) (iOS, Android)

2.  **Add Server**:
    -   Open the app.
    -   When prompted for the **Server URL**, enter your Rustchat server address.
    -   **Example (Localhost)**: `http://localhost:8080`
    -   **Example (LAN)**: `http://192.168.1.50:8080` (Replace with your server's LAN IP)
    -   **Example (Public Domain)**: `https://chat.yourdomain.com` (If configured with a domain)

3.  **Display Name**: Enter a name for the server (e.g., "Rustchat").

4.  **Login**:
    -   Enter your Rustchat credentials (email and password).
    -   *Note: SSO (GitLab/Google/etc.) may not be supported depending on your Rustchat configuration.*

## Troubleshooting

-   **Connection Refused**: Ensure the server is running and the port (8080) is not blocked by a firewall.
-   **WebSocket Disconnected**: If you see a banner about connection issues, ensure that your reverse proxy (if using one other than the provided Nginx) is configured to handle WebSocket upgrades. The provided `frontend/nginx.conf` already handles this.
-   **Invalid SSL**: If you are using HTTPS with a self-signed certificate, you may need to trust the certificate on your device or ignore SSL errors (development only).
