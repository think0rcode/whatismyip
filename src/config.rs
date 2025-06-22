use worker::*;

// Environment variable names
pub const ENV_API_TOKEN: &str = "API_TOKEN";
pub const ENV_CF_ZONE_ID: &str = "CF_ZONE_ID";
pub const ENV_CF_API_TOKEN: &str = "CF_API_TOKEN";
pub const ENV_CF_DOMAIN: &str = "CF_DOMAIN";

/// Application configuration extracted from environment variables
pub struct Config {
    /// Optional API token for request authentication
    pub api_token: Option<String>,
    /// Cloudflare zone ID where DNS records are managed
    pub cf_zone_id: String,
    /// Cloudflare API token for DNS operations
    pub cf_api_token: String,
    /// Domain name to append to hostnames for DNS records
    pub cf_domain: String,
}

impl Config {
    /// Extract configuration from environment variables
    pub fn from_env(env: &Env) -> Result<Self> {
        Ok(Self {
            api_token: env.secret(ENV_API_TOKEN).ok().map(|s| s.to_string()),
            cf_zone_id: env.var(ENV_CF_ZONE_ID)?.to_string(),
            cf_api_token: env.secret(ENV_CF_API_TOKEN)?.to_string(),
            cf_domain: env.var(ENV_CF_DOMAIN)?.to_string(),
        })
    }
}
