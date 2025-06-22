use serde::{Serialize, Deserialize};
use worker::*;

/// Represents the IP address payload returned by the API
#[derive(Serialize)]
struct IpPayload {
    ipv4: String,
    ipv6: String,
}

/// Cloudflare DNS record identifiers stored in KV
#[derive(Deserialize)]
struct DnsRecordInfo {
    record_name: String,
    a_id: Option<String>,
    aaaa_id: Option<String>,
}

/// Supported response formats
#[derive(Debug, PartialEq)]
enum Format {
    Text,
    Json,
    Xml,
}

/// Detects the desired response format from the Accept header
fn detect_format_from_accept(accept_header: Option<&str>) -> Format {
    if let Some(accept) = accept_header {
        let accept = accept.to_lowercase();
        if accept.contains("application/json") {
            return Format::Json;
        }
        if accept.contains("application/xml") || accept.contains("text/xml") {
            return Format::Xml;
        }
    }
    Format::Text
}

/// Detects the desired response format from the request
fn detect_format(req: &Request) -> Format {
    let accept_header = req.headers().get("Accept").ok().flatten();
    detect_format_from_accept(accept_header.as_deref())
}

/// Splits an IP address string into IPv4 and IPv6 components
fn split_ip(ip: &str) -> (String, String) {
    if ip.contains(':') {
        (String::new(), ip.to_string())
    } else if ip.is_empty() {
        (String::new(), String::new())
    } else {
        (ip.to_string(), String::new())
    }
}

/// Validates that the homename only contains ASCII letters, '-' or '_'
fn validate_homename(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '_')
}

/// Formats IP addresses as plain text
fn text_body(ipv4: &str, ipv6: &str) -> String {
    format!("{}\n{}\n", ipv4, ipv6)
}

/// Escapes XML special characters to prevent injection
fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Validates authentication using Bearer token
fn check_auth_with_token(auth_header: Option<&str>, api_token: Option<&str>) -> bool {
    match (api_token, auth_header) {
        (Some(token), Some(auth_header)) if !token.is_empty() && !auth_header.is_empty() => {
            let expected = format!("Bearer {}", token);
            auth_header == expected
        }
        _ => false, // Strict auth: all other cases return false
    }
}

/// Checks authentication against the request and environment
fn check_auth(req: &Request, env: &Env) -> bool {
    let auth_header = req.headers().get("Authorization").ok().flatten();
    let api_token = env.secret("API_TOKEN").ok().map(|t| t.to_string());
    check_auth_with_token(auth_header.as_deref(), api_token.as_deref())
}

/// Updates a Cloudflare DNS record
async fn update_dns_record(
    zone_id: &str,
    token: &str,
    record_id: &str,
    record_type: &str,
    name: &str,
    content: &str,
) -> Result<bool> {
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
        zone_id, record_id
    );
    let body = serde_json::json!({
        "type": record_type,
        "name": name,
        "content": content,
        "ttl": 1,
        "proxied": false
    });
    let mut init = RequestInit::new();
    init.with_method(Method::Put);
    init.with_body(Some(body.to_string().into()));
    let mut req = Request::new_with_init(&url, &init)?;
    req
        .headers_mut()?
        .set("Authorization", &format!("Bearer {}", token))?;
    req.headers_mut()?.set("Content-Type", "application/json")?;
    let mut resp = Fetch::Request(req).send().await?;
    let value: serde_json::Value = resp.json().await?;
    Ok(value
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

/// Checks KV for stored IP and updates DNS if necessary
async fn maybe_update_dns(homename: &str, ipv4: &str, ipv6: &str, env: &Env) -> Result<()> {
    let kv = env.kv("IP_STORE")?;
    let dns_key = format!("{}_dns_rocord_id", homename);
    let dns_info_value = match kv.get(&dns_key).text().await? {
        Some(v) => v,
        None => return Ok(()),
    };
    let dns_info: DnsRecordInfo = serde_json::from_str(&dns_info_value)
        .map_err(|e| Error::RustError(format!("Failed to parse dns info: {}", e)))?;
    let zone_id = env.var("CF_ZONE_ID")?.to_string();
    let token = env.secret("CF_API_TOKEN")?.to_string();

    if !ipv4.is_empty() {
        let key = format!("{}_v4", homename);
        let prev = kv.get(&key).text().await?.unwrap_or_default();
        if prev != ipv4 {
            if let Some(id) = dns_info.a_id.as_deref() {
                if update_dns_record(&zone_id, &token, id, "A", &dns_info.record_name, ipv4).await? {
                    kv.put(&key, ipv4)?.execute().await?;
                }
            }
        }
    }

    if !ipv6.is_empty() {
        let key = format!("{}_v6", homename);
        let prev = kv.get(&key).text().await?.unwrap_or_default();
        if prev != ipv6 {
            if let Some(id) = dns_info.aaaa_id.as_deref() {
                if update_dns_record(&zone_id, &token, id, "AAAA", &dns_info.record_name, ipv6).await? {
                    kv.put(&key, ipv6)?.execute().await?;
                }
            }
        }
    }

    Ok(())
}

/// Creates a response in the specified format
async fn respond(format: Format, ipv4: String, ipv6: String) -> Result<Response> {
    match format {
        Format::Text => Response::ok(text_body(&ipv4, &ipv6)),
        Format::Json => Response::from_json(&IpPayload { ipv4, ipv6 }),
        Format::Xml => {
            let ipv4_escaped = escape_xml(&ipv4);
            let ipv6_escaped = escape_xml(&ipv6);
            let body = format!(
                "<ip><ipv4>{}</ipv4><ipv6>{}</ipv6></ip>",
                ipv4_escaped, ipv6_escaped
            );
            let mut resp = Response::ok(body)?;
            resp.headers_mut().set("Content-Type", "application/xml")?;
            Ok(resp)
        }
    }
}

/// Main request handler
pub async fn handler(req: Request, env: Env) -> Result<Response> {
    if !check_auth(&req, &env) {
        return Response::error("Unauthorized", 401);
    }

    let url = req.url()?;
    let homename = match url.query_pairs().find(|(k, _)| k == "homename") {
        Some((_, value)) => value.to_string(),
        None => return Response::error("homename parameter required", 400),
    };
    if !validate_homename(&homename) {
        return Response::error("invalid homename", 400);
    }

    let ip = req
        .headers()
        .get("CF-Connecting-IP")?
        .unwrap_or_default();

    let (ipv4, ipv6) = split_ip(&ip);
    maybe_update_dns(&homename, &ipv4, &ipv6, &env).await?;
    let fmt = detect_format(&req);

    respond(fmt, ipv4, ipv6).await
}

/// Cloudflare Workers entry point
#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    handler(req, env).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_formatting() {
        assert_eq!(text_body("1.1.1.1", ""), "1.1.1.1\n\n");
        assert_eq!(text_body("", "::1"), "\n::1\n");
        assert_eq!(text_body("1.1.1.1", "::1"), "1.1.1.1\n::1\n");
        assert_eq!(text_body("", ""), "\n\n");
    }

    #[test]
    fn ip_splitting() {
        let test_cases = vec![
            ("1.2.3.4", "1.2.3.4", "", "IPv4 address"),
            ("::1", "", "::1", "IPv6 loopback"),
            (
                "2001:db8:85a3::8a2e:370:7334",
                "",
                "2001:db8:85a3::8a2e:370:7334",
                "Full IPv6 address",
            ),
            ("203.0.113.1", "203.0.113.1", "", "IPv4 test address"),
            ("", "", "", "Empty string"),
        ];

        for (input, expected_v4, expected_v6, description) in test_cases {
            let (actual_v4, actual_v6) = split_ip(input);
            assert_eq!(actual_v4, expected_v4, "IPv4 failed for: {}", description);
            assert_eq!(actual_v6, expected_v6, "IPv6 failed for: {}", description);
        }
    }

    #[test]
    fn detect_format_test_cases() {
        let test_cases = vec![
            (None, Format::Text, "defaults to text when no header"),
            (
                Some("application/json"),
                Format::Json,
                "detects JSON from accept header",
            ),
            (
                Some("text/html,application/json,*/*"),
                Format::Json,
                "finds JSON in mixed accept headers",
            ),
            (
                Some("application/xml"),
                Format::Xml,
                "detects XML application type",
            ),
            (Some("text/xml"), Format::Xml, "detects XML text type"),
            (
                Some("APPLICATION/JSON"),
                Format::Json,
                "handles case insensitive headers",
            ),
            (
                Some("application/xml,application/json"),
                Format::Json,
                "JSON has priority over XML",
            ),
            (
                Some("text/html,image/png"),
                Format::Text,
                "fallback to text for unrecognized types",
            ),
            (
                Some("text/plain"),
                Format::Text,
                "text/plain returns text format",
            ),
            (
                Some("application/pdf"),
                Format::Text,
                "unknown application type returns text",
            ),
            (Some(""), Format::Text, "empty string returns text format"),
            (
                Some("application/json; charset=utf-8"),
                Format::Json,
                "JSON with charset parameter",
            ),
            (
                Some("application/xml; charset=utf-8"),
                Format::Xml,
                "XML with charset parameter",
            ),
        ];

        for (accept_header, expected_format, description) in test_cases {
            let actual_format = detect_format_from_accept(accept_header);
            assert_eq!(
                actual_format, expected_format,
                "Failed test case: {}",
                description
            );
        }
    }

    #[test]
    fn xml_escaping() {
        let test_cases = vec![
            ("normal text", "normal text", "plain text unchanged"),
            ("<script>", "&lt;script&gt;", "angle brackets escaped"),
            ("&amp;", "&amp;amp;", "ampersand escaped"),
            ("\"quoted\"", "&quot;quoted&quot;", "double quotes escaped"),
            ("'single'", "&apos;single&apos;", "single quotes escaped"),
            (
                "192.168.1.1<script>&alert('xss')</script>",
                "192.168.1.1&lt;script&gt;&amp;alert(&apos;xss&apos;)&lt;/script&gt;",
                "complex XSS attempt escaped",
            ),
        ];

        for (input, expected, description) in test_cases {
            assert_eq!(escape_xml(input), expected, "Failed: {}", description);
        }
    }

    #[test]
    fn xml_response_formatting() {
        let test_cases = vec![
            (
                "192.168.1.1",
                "2001:db8::1",
                "<ip><ipv4>192.168.1.1</ipv4><ipv6>2001:db8::1</ipv6></ip>",
                "both IPv4 and IPv6",
            ),
            (
                "",
                "",
                "<ip><ipv4></ipv4><ipv6></ipv6></ip>",
                "empty IPs",
            ),
            (
                "10.0.0.1",
                "",
                "<ip><ipv4>10.0.0.1</ipv4><ipv6></ipv6></ip>",
                "IPv4 only",
            ),
            (
                "",
                "fe80::1",
                "<ip><ipv4></ipv4><ipv6>fe80::1</ipv6></ip>",
                "IPv6 only",
            ),
        ];

        for (ipv4, ipv6, expected, description) in test_cases {
            let ipv4_escaped = escape_xml(ipv4);
            let ipv6_escaped = escape_xml(ipv6);
            let actual = format!(
                "<ip><ipv4>{}</ipv4><ipv6>{}</ipv6></ip>",
                ipv4_escaped, ipv6_escaped
            );
            assert_eq!(actual, expected, "Failed: {}", description);
        }
    }

    #[test]
    fn json_payload_serialization() {
        let test_cases = vec![
            ("192.168.1.1", "2001:db8::1", "both IPs present"),
            ("", "", "empty IPs"),
            ("10.0.0.1", "", "IPv4 only"),
            ("", "fe80::1", "IPv6 only"),
        ];

        for (ipv4, ipv6, description) in test_cases {
            let payload = IpPayload {
                ipv4: ipv4.to_string(),
                ipv6: ipv6.to_string(),
            };
            let json = serde_json::to_string(&payload).unwrap();
            assert!(
                json.contains(&format!("\"ipv4\":\"{}\"", ipv4)),
                "IPv4 missing for: {}",
                description
            );
            assert!(
                json.contains(&format!("\"ipv6\":\"{}\"", ipv6)),
                "IPv6 missing for: {}",
                description
            );
        }
    }

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
                "empty api token - strict auth denies",
            ),
            (
                None,
                Some(""),
                false,
                "empty api token, no header - strict auth denies",
            ),
        ];

        for (auth_header, api_token, expected, description) in test_cases {
            let result = check_auth_with_token(auth_header, api_token);
            assert_eq!(result, expected, "Failed test case: {}", description);
        }
    }

    #[test]
    fn homename_validation() {
        assert!(validate_homename("home"));
        assert!(validate_homename("home-name"));
        assert!(validate_homename("home_name"));
        assert!(!validate_homename(""));
        assert!(!validate_homename("home123"));
        assert!(!validate_homename("home!"));
    }
}
