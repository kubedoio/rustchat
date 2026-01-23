# RustChat Admin Console

The Admin Console provides system administrators with tools to manage users, configure settings, monitor system health, and ensure compliance.

## Accessing the Admin Console

### Requirements
- User must have `system_admin` or `org_admin` role
- Must be authenticated (logged in)

### URL
Navigate to: `http://localhost:8080/admin`

---

## Admin Sections

### 1. Dashboard (`/admin`)
System overview with:
- Total users, active users, teams, channels
- Messages in last 24 hours
- System health status (database, storage, WebSocket)

### 2. Users Management (`/admin/users`)
- View all users with search and filter (by status, role)
- Create new users via "Add User" button
- Deactivate/reactivate users
- Change user roles

### 3. Server Settings (`/admin/server`)
- **Site Configuration**: Name, description, URL
- **File Uploads**: Max file size
- **Localization**: Default locale and timezone

### 4. Security Settings (`/admin/security`)
- Authentication methods (email/password, SSO)
- Password policy (length, complexity requirements)
- SSO/OIDC configuration

### 5. Integrations (`/admin/integrations`)
- Enable/disable webhooks, slash commands, bots

### 6. Compliance (`/admin/compliance`)
- Message retention policy (days)
- File retention policy (days)

### 7. Audit Logs (`/admin/audit`)
- View administrative actions with timestamps
- Filter by action type, date range

### 8. Email Settings (`/admin/email`)
- SMTP configuration (host, port, credentials)
- Test email functionality

### 9. System Health (`/admin/health`)
- Real-time system status
- Database connection and latency
- Storage status
- WebSocket connections count

---

## API Endpoints

All admin endpoints require authentication with admin role.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/admin/config` | Get server configuration |
| PATCH | `/api/v1/admin/config/{category}` | Update config category |
| GET | `/api/v1/admin/users` | List users with filters |
| POST | `/api/v1/admin/users` | Create user |
| PATCH | `/api/v1/admin/users/{id}` | Update user |
| POST | `/api/v1/admin/users/{id}/deactivate` | Deactivate user |
| POST | `/api/v1/admin/users/{id}/reactivate` | Reactivate user |
| GET | `/api/v1/admin/stats` | Get system statistics |
| GET | `/api/v1/admin/health` | Get health status |
| GET | `/api/v1/admin/audit` | Get audit logs |

---

## Creating an Admin User

### Via SQL (first admin)
```sql
UPDATE users SET role = 'system_admin' WHERE email = 'your@email.com';
```

### Via Admin Console
1. Go to Users Management (`/admin/users`)
2. Click "Add User"
3. Fill in details and select role: "System Admin" or "Org Admin"
4. Click "Create User"

---

## SSO Configuration

1. Go to Security Settings (`/admin/security`)
2. Enable "Allow SSO Login"
3. Configure your identity provider in the database:
   ```sql
   INSERT INTO sso_configs (provider, display_name, issuer_url, client_id, client_secret_encrypted, enabled)
   VALUES ('google', 'Google', 'https://accounts.google.com', 'your-client-id', 'your-secret', true);
   ```
4. SSO buttons will appear on the login page automatically
