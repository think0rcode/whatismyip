use worker::*;

// Module declarations
mod auth;
mod config;
mod dns;
mod ip;
mod request;
mod response;
mod service;

// Re-export public APIs
pub use auth::AuthUtils;
pub use config::Config;
pub use dns::DnsManager;
pub use ip::IpUtils;
pub use request::{Format, RequestContext};
pub use response::{IpPayload, ResponseUtils};
pub use service::DnsUpdateService;

// HTTP status codes
const HTTP_UNAUTHORIZED: u16 = 401;
const HTTP_BAD_REQUEST: u16 = 400;

/// Main request handler
pub async fn handler(req: Request, env: Env) -> Result<Response> {
    // Extract configuration
    let config = Config::from_env(&env)?;

    // Check authentication
    if !AuthUtils::check_auth(&req, &config) {
        return Response::error("Unauthorized", HTTP_UNAUTHORIZED);
    }

    // Parse request context
    let ctx = match RequestContext::from_request(&req) {
        Ok(ctx) => ctx,
        Err(e) => return Response::error(e.to_string(), HTTP_BAD_REQUEST),
    };

    // Split IP into IPv4 and IPv6 components
    let (ipv4, ipv6) = IpUtils::split_ip(&ctx.client_ip);

    // Update DNS records if necessary
    if let Err(e) =
        DnsUpdateService::maybe_update_dns(&ctx.homename, &ipv4, &ipv6, &env, &config).await
    {
        // Log error but don't fail the request
        console_log!("DNS update failed: {}", e);
    }

    // Create and return response
    ResponseUtils::create_response(ctx.format, ipv4, ipv6).await
}

/// Cloudflare Workers entry point
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    handler(req, env).await
}
