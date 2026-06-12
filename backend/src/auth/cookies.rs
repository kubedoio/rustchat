//! Authentication cookie helpers.

pub const AUTH_COOKIE_NAME: &str = "MMAUTHTOKEN";

pub fn build_auth_cookie(token: &str, max_age: u64, secure: bool) -> String {
    format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly{}; SameSite=Lax",
        AUTH_COOKIE_NAME,
        token,
        max_age,
        secure_suffix(secure)
    )
}

pub fn clear_auth_cookie(secure: bool) -> String {
    format!(
        "{}=; Path=/; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly{}; SameSite=Lax",
        AUTH_COOKIE_NAME,
        secure_suffix(secure)
    )
}

fn secure_suffix(secure: bool) -> &'static str {
    if secure {
        "; Secure"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_cookie_includes_security_attributes() {
        let cookie = build_auth_cookie("token123", 3600, true);
        assert!(cookie.contains("MMAUTHTOKEN=token123"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=3600"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Secure"));
    }

    #[test]
    fn clear_auth_cookie_expires_cookie() {
        let cookie = clear_auth_cookie(true);
        assert!(cookie.contains("MMAUTHTOKEN="));
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Secure"));
    }
}
