use crate::config::Config;
use worker::*;

// Constants
const HEADER_AUTHORIZATION: &str = "Authorization";
const BEARER_PREFIX: &str = "Bearer ";

/// Authentication utilities
pub struct AuthUtils;

impl AuthUtils {
    /// Checks authentication against the request and environment
    pub fn check_auth(req: &Request, config: &Config) -> bool {
        let auth_header = req.headers().get(HEADER_AUTHORIZATION).ok().flatten();
        Self::check_auth_with_token(auth_header.as_deref(), config.api_token.as_deref())
    }

    /// Validates authentication using Bearer token
    pub fn check_auth_with_token(auth_header: Option<&str>, api_token: Option<&str>) -> bool {
        match (api_token, auth_header) {
            (Some(token), Some(auth_header)) if !token.is_empty() && !auth_header.is_empty() => {
                let expected = format!("{}{}", BEARER_PREFIX, token);
                auth_header == expected
            }
            _ => false, // Strict auth: all other cases return false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_auth_test_cases() {
        let test_cases = vec![
            (
                None,
                None,
                false,
                "no token configured, no auth header - strict auth denies",
            ),
            (
                Some("Bearer test123"),
                None,
                false,
                "no token configured, with auth header - strict auth denies",
            ),
            (
                Some("Bearer test123"),
                Some(""),
                false,
                "empty token configured - strict auth denies",
            ),
            (
                None,
                Some("secret"),
                false,
                "token configured, no auth header - strict auth denies",
            ),
            (
                Some("Bearer secret"),
                Some("secret"),
                true,
                "exact token match - should allow",
            ),
            (
                Some("Bearer wrong"),
                Some("secret"),
                false,
                "wrong token - strict auth denies",
            ),
            (
                Some("Basic secret"),
                Some("secret"),
                false,
                "wrong auth scheme - strict auth denies",
            ),
            (
                Some("Bearer "),
                Some("secret"),
                false,
                "empty bearer token - strict auth denies",
            ),
            (
                Some(""),
                Some("secret"),
                false,
                "empty auth header - strict auth denies",
            ),
            (
                Some("Bearer secret extra"),
                Some("secret"),
                false,
                "token with extra data - strict auth denies",
            ),
            (
                Some("bearer secret"),
                Some("secret"),
                false,
                "lowercase bearer - strict auth denies",
            ),
            (
                Some("Bearer secret"),
                Some("SECRET"),
                false,
                "token case mismatch - strict auth denies",
            ),
            (
                Some("Bearer secret123"),
                Some("secret123"),
                true,
                "exact alphanumeric token match - should allow",
            ),
            (
                Some("Bearer secret-token_123"),
                Some("secret-token_123"),
                true,
                "exact complex token match - should allow",
            ),
            (
                Some("Bearer x"),
                Some("y"),
                false,
                "different minimal tokens - strict auth denies",
            ),
            (
                Some("Bearer secret"),
                Some(""),
                false,
                "empty configured token - strict auth denies",
            ),
        ];

        for (auth_header, api_token, expected, description) in test_cases {
            let result = AuthUtils::check_auth_with_token(auth_header, api_token);
            assert_eq!(result, expected, "Failed: {}", description);
        }
    }
}
