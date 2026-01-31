#!/usr/bin/env python3
import requests
import json
import argparse
import sys
import uuid

def main():
    parser = argparse.ArgumentParser(description="Live compatibility smoke test")
    parser.add_argument("--url", default="https://app.rustchat.io", help="Target server URL")
    parser.add_argument("--username", required=True, help="Username/Email")
    parser.add_argument("--password", required=True, help="Password")
    args = parser.parse_args()

    base_url = args.url.rstrip('/')
    print(f"--- Running Expanded Compatibility Smoke Test against {base_url} ---")

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
    except Exception as e:
        print(f"ERROR: {e}")
        sys.exit(1)

    headers = {
        "Authorization": f"Bearer {token}",
        "X-Requested-With": "XMLHttpRequest"
    }

    # 2. Client Config (legacy format)
    print(f"Step 2: GET /api/v4/config/client?format=old ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/config/client?format=old", headers=headers)
    if resp.status_code == 200:
        print("SUCCESS")
    else:
        print(f"FAILED ({resp.status_code})")
        sys.exit(1)

    # 3. Get Me
    print(f"Step 2: GET /api/v4/users/me ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/users/me", headers=headers)
    if resp.status_code == 200:
        me = resp.json()
        user_id = me.get('id')
        username = me.get('username')
        print(f"SUCCESS (Username: {username})")
    else:
        print(f"FAILED ({resp.status_code})")
        sys.exit(1)

    # 4. Get Teams & Channels
    print(f"Step 4: Discovering Workspace ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/users/me/teams", headers=headers)
    if resp.status_code == 200:
        teams = resp.json()
        if not teams:
            print("FAILED (No teams found)")
            sys.exit(1)
        
        team_id = teams[0]['id']
        resp = requests.get(f"{base_url}/api/v4/users/me/teams/{team_id}/channels", headers=headers)
        if resp.status_code == 200:
            channels = resp.json()
            if not channels:
                 print("FAILED (No channels found in team)")
                 sys.exit(1)
            
            # Find a non-direct channel to post to
            channel = next((c for c in channels if c['type'] in ['O', 'P']), channels[0])
            channel_id = channel['id']
            print(f"SUCCESS (Team: {team_id}, Target Channel: {channel['display_name']})")
        else:
            print(f"FAILED (Channels: {resp.status_code})")
            sys.exit(1)
    else:
        print(f"FAILED (Teams: {resp.status_code})")
        sys.exit(1)

    post_id = None

    # 5. Messaging (Phase 3)
    test_msg = f"In-situ compatibility smoke test {uuid.uuid4().hex[:8]}"
    print(f"Step 4: POST /api/v4/posts (Create Message) ... ", end="")
    payload = {
        "channel_id": channel_id,
        "message": test_msg
    }
    resp = requests.post(f"{base_url}/api/v4/posts", headers=headers, json=payload)
    if resp.status_code in [200, 201]:
        post = resp.json()
        post_id = post['id']
        print(f"SUCCESS (Post ID: {post_id})")
        
        print(f"        GET /api/v4/channels/{channel_id}/posts (Verify) ... ", end="")
        resp = requests.get(f"{base_url}/api/v4/channels/{channel_id}/posts", headers=headers)
        if resp.status_code == 200:
            posts = resp.json().get('posts', {})
            if any(p['message'] == test_msg for p in posts.values()):
                print("SUCCESS")
            else:
                print("FAILED (Post not found in feed)")
        else:
             print(f"FAILED ({resp.status_code})")
    else:
        print(f"FAILED ({resp.status_code})")

    # 6. Advanced Search (Phase 4)
    print(f"Step 5: POST /api/v4/users/search ... ", end="")
    payload = {"term": username}
    resp = requests.post(f"{base_url}/api/v4/users/search", headers=headers, json=payload)
    if resp.status_code == 200:
        results = resp.json()
        if any(u['id'] == user_id for u in results):
             print("SUCCESS (User found)")
        else:
             print("FAILED (User not found in search results)")
    else:
        print(f"FAILED ({resp.status_code})")

    # 7. Preferences (Phase 4)
    print(f"Step 6: GET /api/v4/users/me/preferences ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/users/me/preferences", headers=headers)
    if resp.status_code == 200:
        prefs = resp.json()
        print(f"SUCCESS ({len(prefs)} preference entries)")
    else:
        print(f"FAILED ({resp.status_code})")

    print(f"        PUT /api/v4/users/{user_id}/preferences ... ", end="")
    prefs_payload = [
        {
            "user_id": user_id,
            "category": "test",
            "name": "smoke",
            "value": "true",
        }
    ]
    resp = requests.put(
        f"{base_url}/api/v4/users/{user_id}/preferences",
        headers=headers,
        json=prefs_payload,
    )
    if resp.status_code == 200:
        print("SUCCESS")
    else:
        print(f"FAILED ({resp.status_code})")

    # 8. Threads (Phase 3)
    print(f"Step 8: GET /api/v4/users/me/threads ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/users/me/threads", headers=headers)
    if resp.status_code == 200:
        print("SUCCESS")
    else:
        print(f"FAILED ({resp.status_code})")

    # 9. Reactions (Phase 6)
    if not post_id:
        print("Step 9: Social Interaction - Reactions ... SKIPPED (no post id)")
    else:
        print(f"Step 9: Social Interaction - Reactions ... ", end="")
        payload = {
            "user_id": user_id,
            "post_id": post_id,
            "emoji_name": "thumbsup"
        }
        resp = requests.post(f"{base_url}/api/v4/reactions", headers=headers, json=payload)
        if resp.status_code == 200:
            print("ADDED ... ", end="")
            resp = requests.get(f"{base_url}/api/v4/posts/{post_id}/reactions", headers=headers)
            if resp.status_code == 200 and any(r['emoji_name'] == 'thumbsup' for r in resp.json()):
                print("VERIFIED ... ", end="")
                resp = requests.delete(f"{base_url}/api/v4/users/{user_id}/posts/{post_id}/reactions/thumbsup", headers=headers)
                if resp.status_code == 200:
                    print("REMOVED SUCCESS")
                else:
                    print(f"REMOVE FAILED ({resp.status_code})")
            else:
                print(f"VERIFY FAILED ({resp.status_code})")
        else:
            print(f"ADD FAILED ({resp.status_code})")

    # 10. Emojis (Phase 6)
    print(f"Step 10: Social Interaction - Emojis ... ", end="")
    resp = requests.get(f"{base_url}/api/v4/emoji", headers=headers)
    if resp.status_code == 200:
        emojis = resp.json()
        print(f"SUCCESS ({len(emojis)} custom emojis found)")
    else:
        print(f"FAILED ({resp.status_code})")

    print("\n--- Expanded Smoke Test Completed ---")

if __name__ == "__main__":
    main()
