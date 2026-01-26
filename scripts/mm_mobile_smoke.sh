#!/bin/bash
set -e

# Configuration
BASE=${BASE:-http://localhost:3000}
TOKEN=${TOKEN:-""}

echo "=== RustChat Mattermost Compatibility Smoke Test ==="
echo "Target: $BASE"

check_header() {
    local url=$1
    local header=$2
    echo "Checking $url for header $header..."
    local out=$(curl -s -I "$url")
    if echo "$out" | grep -qi "$header"; then
        echo "  [OK] Header found"
    else
        echo "  [FAIL] Header $header missing"
        echo "Response headers:"
        echo "$out"
        exit 1
    fi
}

check_json() {
    local url=$1
    local field=$2
    echo "Checking $url for JSON field $field..."
    local out=$(curl -s "$url")

    # Check for HTML (SPA fallback)
    if echo "$out" | grep -qi "<html"; then
        echo "  [FAIL] Received HTML instead of JSON (SPA fallback suspected)"
        exit 1
    fi

    if echo "$out" | grep -q "\"$field\""; then
        echo "  [OK] Field found"
    else
        echo "  [FAIL] Field $field missing in JSON"
        echo "Response:"
        echo "$out"
        exit 1
    fi
}

check_exact() {
    local url=$1
    local expected=$2
    echo "Checking $url content..."
    local out=$(curl -s "$url")
    if [ "$out" == "$expected" ]; then
        echo "  [OK] Content matches"
    else
        echo "  [FAIL] Expected '$expected', got '$out'"
        exit 1
    fi
}

# 1) Verify routing
check_header "$BASE/api/v4/system/ping" "X-MM-COMPAT: 1"

# 2) Handshake
check_json "$BASE/api/v4/system/ping" "status"
check_json "$BASE/api/v4/config/client?format=old" "Version"
check_json "$BASE/api/v4/license/client?format=old" "IsLicensed"

echo "Checking system version..."
VERSION_OUT=$(curl -s "$BASE/api/v4/system/version")
if [[ "$VERSION_OUT" == *"10.11.0"* ]]; then
    echo "  [OK] Version is 10.11.0"
else
    echo "  [FAIL] Version mismatch: $VERSION_OUT"
    exit 1
fi

# 3) Login + me
if [ -n "$TOKEN" ]; then
    echo "Checking /users/me..."
    curl -s "$BASE/api/v4/users/me" -H "Authorization: Bearer $TOKEN" | grep "id" || { echo "  [FAIL] /users/me failed"; exit 1; }
    echo "  [OK] /users/me success"
else
    echo "Skipping /users/me check (TOKEN not set)"
fi

# 4) Check Not Implemented
echo "Checking unimplemented endpoint..."
NOT_IMPL_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/v4/users/unknown/endpoint")
if [ "$NOT_IMPL_CODE" == "501" ]; then
    echo "  [OK] Returns 501"
else
    echo "  [FAIL] Expected 501, got $NOT_IMPL_CODE"
    # Don't fail script if it returns 404 for now, but user requirement said 501.
    # If my routing fallback works, it should be 501.
    # But wait, if I request a path that DOES NOT MATCH any route in `users` router?
    # `users` router is merged into `v4` router.
    # If no route matches in `v4` router, it falls back to `not_implemented` defined in `api/v4/mod.rs`.
    # So it SHOULD be 501.
    exit 1
fi

echo "=== All Tests Passed ==="
