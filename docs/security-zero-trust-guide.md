# Security Zero-Trust Guide

This guide applies zero-trust networking principles to RustChat deployments. In a zero-trust model, no user, device, or service is implicitly trusted — every access request is authenticated, authorized, and encrypted.

## Target Audience

Security-conscious operators, infrastructure engineers, and compliance teams running RustChat in regulated or high-threat environments.

---

## Network Segmentation

### Principle

No data store or internal service should be directly reachable from the public internet. Assume breach: if an attacker gains access to one component, they should not automatically gain access to everything else.

### RustChat Architecture Segments

```
┌─────────────────┐
│   Internet      │
│  (untrusted)    │
└────────┬────────┘
         │
    ┌────┴────┐
    │  WAF /  │
    │  LB     │
    └────┬────┘
         │
┌────────┴────────┐
│  Reverse Proxy  │  ← TLS termination, rate limiting, IP allowlisting
│  (DMZ/trusted)  │
└────────┬────────┘
         │
    ┌────┴────┐
    │RustChat │  ← Application layer
    │ Backend │
    └────┬────┘
         │
    ┌────┴────┐
    │  Data   │
    │  Tier   │  ← PostgreSQL, Redis, S3-compatible storage (private subnet only)
    └─────────┘
```

### Implementation

1. **Database (PostgreSQL):** Bind to a private IP or Unix socket. Block port `5432` at the host and network firewall. RustChat should be the only service that can reach it.
2. **Redis:** Bind to `127.0.0.1` (single node) or a private VPC subnet (clustered). Never expose `6379` publicly.
3. **S3-compatible storage:** Run RustFS on an internal network or use a cloud VPC endpoint. If public access to uploaded files is required, serve them through a CDN or signed-URL proxy, not direct bucket access.
4. **Push Proxy:** If running as a sidecar, bind to `localhost` or an internal Docker network. The push proxy should not accept requests from the internet.

---

## Authentication Everywhere

### No Implicit Trust Between Services

| Connection | Authentication Mechanism |
|------------|-------------------------|
| Client → Backend | JWT (Argon2-hashed passwords) |
| Backend → PostgreSQL | Username/password + TLS |
| Backend → Redis | Redis AUTH + TLS |
| Backend → S3 | HMAC access/secret keys |
| Backend → Push Proxy | Shared secret (`PUSH_PROXY_AUTH_KEY`) |
| Push Proxy → FCM | Service account JWT |
| Push Proxy → APNS | Apple JWT (p8 key) |

### Service-to-Service Checklist

- [ ] Every internal connection uses credentials, not just IP allowlisting.
- [ ] Credentials are unique per service pair (do not reuse the same Redis password for unrelated applications).
- [ ] mTLS is preferred where supported (PostgreSQL client certs, Redis TLS with client verification).

---

## API Key and Token Lifecycle Management

### JWT Tokens

- **Expiry:** Default is 24 hours (`RUSTCHAT_JWT_EXPIRY_HOURS`). In high-security environments, reduce this to 4–8 hours and require re-authentication.
- **Refresh:** RustChat does not implement refresh tokens natively. Users must re-login when JWTs expire. For long-lived mobile sessions, ensure devices are enrolled and trackable.
- **Revocation:** Changing `RUSTCHAT_JWT_SECRET` revokes all tokens globally. There is no per-user revocation mechanism; plan incident response accordingly.

### S3 Credentials

- Rotate access keys every 90 days.
- Use IAM-style policies scoped to a single bucket and action set.
- Avoid long-lived credentials. If your S3 provider supports STS or presigned URLs, prefer those for external integrations.

### Push Proxy Secrets

- `PUSH_PROXY_AUTH_KEY` should be rotated if any backend instance is compromised.
- APNS p8 keys do not expire, but can be revoked in the Apple Developer Portal.

---

## Principle of Least Privilege for Users and Roles

### Built-in Roles

RustChat supports role-based access control. Default roles include:

- `system_admin`: Full access. Minimize the number of accounts with this role.
- `team_admin`: Team-level administration.
- `user`: Standard participation.

### Recommendations

1. **Grant `system_admin` to no more than 2–3 accounts.** Use named accounts, not shared credentials.
2. **Audit role assignments quarterly.** Remove elevated roles from users who no longer need them.
3. **Disable or delete stale accounts.** Inactive accounts are attack surface.
4. **Use channel-level privacy settings.** Default sensitive channels to private.

---

## Session Management and Timeout Policies

### Configuration

- Set `RUSTCHAT_JWT_EXPIRY_HOURS` to match your organization's session policy.
- In production, enforce short sessions and require password re-entry for sensitive actions (admin settings, user deletion).

### Best Practices

1. **Invalidate sessions on password change.** When a user resets their password, all existing JWTs should be considered invalid. Since RustChat does not maintain a token blocklist, this requires rotating `RUSTCHAT_JWT_SECRET` or accepting the limitation until a session revocation feature is added.
2. **Detect anomalous sessions.** Monitor for:
   - Logins from new geolocations or ASNs
   - Multiple concurrent sessions from different regions
   - WebSocket connections without preceding HTTP authentication
3. **Require MFA where available.** If your identity provider supports OAuth2/OIDC with MFA, prefer that over local password authentication.

---

## Forward Secrecy and TLS Best Practices

### TLS Configuration

1. **TLS 1.3 only.** Disable TLS 1.0, 1.1, and 1.2 cipher suites that lack forward secrecy.
2. **Use ECDSA or RSA certificates with strong key lengths** (P-256 or RSA 2048+).
3. **Enable OCSP stapling** on your reverse proxy for faster revocation checks.
4. **HSTS header:**
   ```
   Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
   ```

### Forward Secrecy

Ensure your TLS configuration uses ephemeral key exchange (ECDHE). This prevents past traffic from being decrypted if the server's private key is later compromised.

### Internal TLS

Where possible, encrypt traffic between internal services:
- Backend → PostgreSQL (`sslmode=require`)
- Backend → Redis (`rediss://`)
- Backend → S3-compatible storage (HTTPS endpoint)

---

## Monitoring and Anomaly Detection

### Key Metrics to Alert On

| Signal | Threshold | Indication |
|--------|-----------|------------|
| Failed login rate | > 5/min per IP | Brute-force attempt |
| JWT validation failures | Spike above baseline | Token replay or tampering |
| 403/401 rate | > 10% of total | Credential stuffing or scanning |
| WebSocket connection rate | > 100/min per IP | Possible DoS or bot |
| File upload volume | > 3x baseline | Data exfiltration or abuse |
| Admin action count | Unusual off-hours | Compromised admin account |

### Log Correlation

Centralize logs from:
- Reverse proxy (access logs)
- RustChat backend (application logs)
- PostgreSQL (connection logs, slow queries)
- Redis (command logs in ACL mode)
- S3-compatible storage (access logs)

Use correlation rules to detect multi-stage attacks (e.g., login failure followed by successful login from a new IP, followed by admin action).

### Tooling

- **Open source:** Wazuh, Falco, Suricata
- **Commercial:** Datadog Security Monitoring, Splunk ES, Elastic Security
- **Cloud-native:** AWS GuardDuty, GCP Security Command Center

---

## Incident Response Basics for Self-Hosted Operators

### Preparation

1. **Maintain an offline contact list** for your team and hosting provider.
2. **Document your RustChat deployment topology** (versions, hostnames, backup locations).
3. **Keep GPG-encrypted backups** of critical secrets in a separate location.

### Detection

1. **Set up real-time alerting** on the signals listed above.
2. **Review logs daily** during the first weeks of production deployment.

### Containment

| Scenario | Immediate Action |
|----------|-----------------|
| Suspected credential leak | Rotate `RUSTCHAT_JWT_SECRET` and all service passwords |
| Confirmed admin account compromise | Demote or disable the account; audit admin logs |
| Database exposure | Block DB port; rotate DB password; check for unauthorized queries |
| File upload abuse | Disable uploads temporarily; review uploaded files |
| DDoS / volumetric attack | Enable Cloudflare or upstream DDoS protection; scale backend horizontally |

### Eradication and Recovery

1. **Identify the root cause** before restoring services. A quick restore without patching the entry point invites reinfection.
2. **Restore from known-good backups** if data integrity is in question.
3. **Rotate all secrets** after a confirmed breach.
4. **Document lessons learned** and update runbooks.

### Communication

- Notify affected users if personal data was accessed.
- Follow your jurisdiction's breach notification laws (GDPR, CCPA, etc.).
- For vulnerabilities in RustChat itself, report privately per `SECURITY.md`.

---

## Summary

Zero trust is not a single product or configuration flag — it is a continuous practice:

1. **Verify explicitly.** Every request is authenticated and authorized.
2. **Use least privilege.** Users and services get only the access they need.
3. **Assume breach.** Segment networks, encrypt data in transit and at rest, and monitor for anomalies.
4. **Rotate and revoke.** Credentials and tokens have a finite lifetime.

For concrete hardening steps (TLS setup, rate limiting, secret generation), see [`docs/security-deployment-guide.md`](security-deployment-guide.md).
