#!/usr/bin/env python3
import requests
import json
import argparse
import sys

def main():
    parser = argparse.ArgumentParser(description="Live compatibility smoke test")
    parser.add_argument("--url", default="https://rustchat.io", help="Target server URL")
    parser.add_argument("--username", required=True, help="Username/Email")
    parser.add_argument("--password", required=True, help="Password")
    args = parser.parse_args()

    base_url = args.url.rstrip('/')
    print(f"--- Running Compatibility Smoke Test against {base_url} ---")

    # 1. Login
    login_url = f"{base_url}/api/v4/users/login"
    payload = {
        "login_id": args.username,
        "password": args.password
    }
    
    print(f"Step 1: POST /api/v4/users/login ... ", end="")
    try:
        resp = requests.post(login_url, json=payload, timeout=10)
        if resp.status_code != 200:
            print(f"FAILED ({resp.status_code})")
            print(resp.text)
            sys.exit(1)
        
        token = resp.headers.get("Token")
        if not token:
            print("FAILED (No Token header in response)")
            sys.exit(1)
            
        print("SUCCESS")
        print(f"  Token: {token[:10]}...")
    except Exception as e:
        print(f"ERROR: {e}")
        sys.exit(1)

    headers = {
        "Authorization": f"Bearer {token}",
        "X-Requested-With": "XMLHttpRequest"
    }

    # 2. Get Me
    print(f"Step 2: GET /api/v4/users/me ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/users/me", headers=headers)
    if resp.status_code == 200:
        me = resp.json()
        print(f"SUCCESS (Username: {me.get('username')})")
    else:
        print(f"FAILED ({resp.status_code})")

    # 3. Get Teams
    print(f"Step 3: GET /api/v4/users/me/teams ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/users/me/teams", headers=headers)
    if resp.status_code == 200:
        teams = resp.json()
        print(f"SUCCESS ({len(teams)} teams found)")
        if teams:
            team_id = teams[0]['id']
            # 4. Get Channels
            print(f"Step 4: GET /api/v4/users/me/teams/{team_id}/channels ... ", end="")
            resp = requests.get(f"{base_url}/api/v4/users/me/teams/{team_id}/channels", headers=headers)
            if resp.status_code == 200:
                channels = resp.json()
                print(f"SUCCESS ({len(channels)} channels found)")
            else:
                print(f"FAILED ({resp.status_code})")
    else:
        print(f"FAILED ({resp.status_code})")

    # 5. Check WebSocket
    print(f"Step 5: Check Compatibility Header ... ", end="")
    # The mappers/router should add X-MM-Compat: 1
    resp = requests.get(f"{base_url}/api/v4/system/ping", headers=headers)
    if resp.headers.get("X-MM-Compat") == "1":
         print("SUCCESS (X-MM-Compat: 1 found)")
    else:
         print("WARNING (X-MM-Compat header missing - check router/middleware)")

    print("\n--- Smoke Test Completed ---")

if __name__ == "__main__":
    main()
