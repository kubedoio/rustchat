# Frontend OAuth Integration Guide

This guide shows how to integrate OAuth authentication using the secure exchange code flow.

## Overview

Instead of receiving the JWT token directly in the URL (which appears in browser history and logs), the new secure flow:

1. User authenticates with OAuth provider
2. Server redirects with a **one-time exchange code**
3. Frontend exchanges the code for a JWT token via POST request
4. Token is stored securely (never in URL)

## Implementation

### React/Vue Example

```typescript
// auth/oauth.ts

interface OAuthConfig {
  baseUrl: string;
}

class OAuthService {
  private baseUrl: string;
  
  constructor(config: OAuthConfig) {
    this.baseUrl = config.baseUrl;
  }
  
  /**
   * Start OAuth flow by redirecting to provider
   */
  startOAuthFlow(provider: string, redirectPath: string = '/') {
    // Store redirect path for after login
    sessionStorage.setItem('oauth_redirect', redirectPath);
    
    // Redirect to OAuth login endpoint
    const loginUrl = `${this.baseUrl}/api/v1/oauth2/${provider}/login`;
    window.location.href = loginUrl;
  }
  
  /**
   * Handle OAuth callback - exchange code for token
   */
  async handleCallback(): Promise<string | null> {
    const urlParams = new URLSearchParams(window.location.search);
    
    // Check for errors
    const error = urlParams.get('error');
    if (error) {
      throw new Error(`OAuth error: ${error}`);
    }
    
    // Secure mode: exchange code for token
    const code = urlParams.get('code');
    if (!code) {
      return null; // Not an OAuth callback
    }
    
    try {
      const token = await this.exchangeCode(code);
      this.storeToken(token);
      
      // Clean up URL (remove code from browser history)
      const redirectPath = sessionStorage.getItem('oauth_redirect') || '/';
      window.history.replaceState({}, document.title, redirectPath);
      sessionStorage.removeItem('oauth_redirect');
      
      return token;
    } catch (err) {
      throw new Error(`Failed to exchange code: ${err.message}`);
    }
  }
  
  /**
   * Exchange one-time code for JWT token
   */
  private async exchangeCode(code: string): Promise<string> {
    const response = await fetch(`${this.baseUrl}/api/v1/oauth2/exchange`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ code }),
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error?.message || 'Exchange failed');
    }
    
    const data = await response.json();
    return data.token;
  }
  
  /**
   * Store token securely
   */
  private storeToken(token: string) {
    // Prefer httpOnly cookie (server-side)
    // Fallback: memory storage (most secure client-side option)
    // Avoid: localStorage (XSS vulnerable), sessionStorage (less secure than memory)
    
    // This example uses memory storage
    // In production, use httpOnly cookies set by the server
    (window as any).__TOKEN__ = token;
  }
  
  /**
   * Get stored token
   */
  getToken(): string | null {
    return (window as any).__TOKEN__ || null;
  }
  
  /**
   * Clear token (logout)
   */
  logout() {
    (window as any).__TOKEN__ = null;
    // Also call server logout endpoint if needed
  }
}

// Singleton instance
export const oauthService = new OAuthService({
  baseUrl: process.env.REACT_APP_API_URL || 'http://localhost:3000'
});

export default OAuthService;
```

### React Hook Example

```typescript
// hooks/useAuth.ts
import { useState, useEffect, useCallback } from 'react';
import { oauthService } from '../auth/oauth';

interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}

export function useAuth() {
  const [state, setState] = useState<AuthState>({
    isAuthenticated: false,
    isLoading: true,
    error: null
  });
  
  // Check for OAuth callback on mount
  useEffect(() => {
    const handleCallback = async () => {
      if (!window.location.search.includes('code=')) {
        setState(s => ({ ...s, isLoading: false }));
        return;
      }
      
      try {
        const token = await oauthService.handleCallback();
        if (token) {
          setState({
            isAuthenticated: true,
            isLoading: false,
            error: null
          });
        }
      } catch (err) {
        setState({
          isAuthenticated: false,
          isLoading: false,
          error: err.message
        });
      }
    };
    
    handleCallback();
  }, []);
  
  const login = useCallback((provider: string) => {
    oauthService.startOAuthFlow(provider);
  }, []);
  
  const logout = useCallback(() => {
    oauthService.logout();
    setState({
      isAuthenticated: false,
      isLoading: false,
      error: null
    });
  }, []);
  
  return {
    ...state,
    login,
    logout,
    token: oauthService.getToken()
  };
}
```

### WebSocket Connection with Secure Handshake Token

```typescript
// websocket/client.ts

class WebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string | null;
  
  constructor(baseUrl: string, token: string | null) {
    // Convert http(s) to ws(s)
    this.url = baseUrl.replace(/^http/, 'ws') + '/api/v1/ws';
    this.token = token;
  }
  
  connect() {
    // Create WebSocket with Authorization in subprotocol
    // This is the secure way (token not in URL)
    const protocols = this.token ? [`token.${this.token}`] : undefined;
    this.ws = new WebSocket(this.url, protocols);
    
    this.ws.onopen = () => {
      console.log('WebSocket connected');
    };
    
    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      this.handleMessage(data);
    };
    
    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
    
    this.ws.onclose = () => {
      console.log('WebSocket closed');
      // Implement reconnection logic here
    };
  }
  
  private handleMessage(data: any) {
    // Handle different message types
    switch (data.event) {
      case 'hello':
        console.log('Server hello:', data.data);
        break;
      case 'posted':
        console.log('New message:', data.data);
        break;
      default:
        console.log('Received:', data);
    }
  }
  
  disconnect() {
    this.ws?.close();
  }
  
  send(data: any) {
    this.ws?.send(JSON.stringify(data));
  }
}

export default WebSocketClient;
```

### Login Component Example

```tsx
// components/Login.tsx
import React from 'react';
import { useAuth } from '../hooks/useAuth';

export function Login() {
  const { login, isLoading, error } = useAuth();
  
  return (
    <div className="login-container">
      <h1>Welcome to RustChat</h1>
      
      {error && (
        <div className="error-banner">
          {error}
        </div>
      )}
      
      <div className="login-buttons">
        <button 
          onClick={() => login('google')}
          disabled={isLoading}
          className="btn btn-google"
        >
          {isLoading ? 'Connecting...' : 'Continue with Google'}
        </button>
        
        <button 
          onClick={() => login('github')}
          disabled={isLoading}
          className="btn btn-github"
        >
          {isLoading ? 'Connecting...' : 'Continue with GitHub'}
        </button>
      </div>
      
      <p className="security-note">
        🔒 Secure OAuth 2.0 with PKCE
      </p>
    </div>
  );
}
```

## Secure Integration Checklist

### 1. Backend Configuration

```bash
# Required secure mode
RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie
```

### 2. Frontend Changes

```typescript
// Exchange code for token
const code = new URLSearchParams(window.location.search).get('code');
const response = await fetch('/api/v1/oauth2/exchange', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ code })
});
const { token } = await response.json();
// Store securely (httpOnly cookie or memory)
```

### 3. WebSocket Changes

```typescript
// Token in subprotocol header
const ws = new WebSocket('wss://api.example.com/ws', [`token.${token}`]);
```

## Security Best Practices

1. **Never store tokens in localStorage/sessionStorage**
   - Use httpOnly cookies (server-side)
   - Or memory-only storage (most secure client-side)

2. **Clear exchange code from URL immediately**
   ```typescript
   window.history.replaceState({}, document.title, '/');
   ```

3. **Validate state parameter**
   - The backend handles CSRF protection via state parameter
   - No additional action needed in frontend

4. **Use PKCE for mobile apps**
   - Backend automatically uses PKCE for OAuth providers
   - Ensures code can only be exchanged by the requesting client

## Testing

### Unit Tests

```typescript
// __tests__/oauth.test.ts
import { oauthService } from '../auth/oauth';

describe('OAuth Service', () => {
  beforeEach(() => {
    // Reset URL
    window.history.pushState({}, '', '/');
    sessionStorage.clear();
  });
  
  test('handles exchange code flow', async () => {
    // Mock fetch
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ token: 'test-token' })
    });
    
    // Simulate callback
    window.history.pushState({}, '', '/?code=abc123');
    
    const token = await oauthService.handleCallback();
    
    expect(token).toBe('test-token');
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/oauth2/exchange'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ code: 'abc123' })
      })
    );
  });
  
  test('clears code from URL after exchange', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ token: 'test-token' })
    });
    
    window.history.pushState({}, '', '/?code=abc123');
    await oauthService.handleCallback();
    
    expect(window.location.search).toBe('');
  });
});
```

## Troubleshooting

### "Invalid or already used exchange code"

- Code has expired (60-second TTL) or already been used
- Check network latency between OAuth provider and your server
- Ensure code is only exchanged once

### "Token not found in WebSocket"

- Verify token is being sent via Sec-WebSocket-Protocol header
- Check browser console for connection errors

### "CSP violation"

- Ensure your Content Security Policy allows the OAuth popup/redirect
- Add OAuth provider domains to CSP if needed
