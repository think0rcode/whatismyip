use worker::*;
use serde::Serialize;
use subtle::ConstantTimeEq;

#[derive(Serialize)]
struct IpPayload {
    ipv4: String,
    ipv6: String,
}

#[derive(Debug, PartialEq)]
enum Format {
    Text,
    Json,
    Xml,
}

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

fn detect_format(req: &Request) -> Format {
    let accept_header = req.headers().get("Accept").ok().flatten();
    detect_format_from_accept(accept_header.as_deref())
}

fn split_ip(ip: &str) -> (String, String) {
    if ip.contains(':') {
        (String::new(), ip.to_string())
    } else if ip.is_empty() {
        (String::new(), String::new())
    } else {
        (ip.to_string(), String::new())
    }
}

fn text_body(ipv4: &str, ipv6: &str) -> String {
    format!("{}\n{}\n", ipv4, ipv6)
}

fn check_auth(req: &Request, env: &Env) -> bool {
    if let Ok(token) = env.secret("API_TOKEN") {
        let expected = format!("Bearer {}", token.to_string());
        match req.headers().get("Authorization").ok().flatten() {
            Some(ref h)
                if h.as_bytes().ct_eq(expected.as_bytes()).into() => true,
            _ => false,
        }
    } else {
        true
    }
}

fn escape_xml(input: &str) -> String {
    input.replace('&', "&amp;")
         .replace('<', "&lt;")
         .replace('>', "&gt;")
         .replace('"', "&quot;")
         .replace('\'', "&apos;")
}

async fn respond(format: Format, ipv4: String, ipv6: String) -> Result<Response> {
    match format {
        Format::Text => Response::ok(text_body(&ipv4, &ipv6)),
        Format::Json => Response::from_json(&IpPayload { ipv4, ipv6 }),
        Format::Xml => {
            let ipv4_escaped = escape_xml(&ipv4);
            let ipv6_escaped = escape_xml(&ipv6);
            let body = format!("<ip><ipv4>{}</ipv4><ipv6>{}</ipv6></ip>", ipv4_escaped, ipv6_escaped);
            let mut resp = Response::ok(body)?;
            resp.headers_mut().set("Content-Type", "application/xml")?;
            Ok(resp)
        }
    }
}

pub async fn handler(req: Request, env: Env) -> Result<Response> {
    if !check_auth(&req, &env) {
        return Response::error("Unauthorized", 401);
    }
    let ip = req
        .headers()
        .get("CF-Connecting-IP")?
        .unwrap_or_default();
    let (ipv4, ipv6) = split_ip(&ip);
    let fmt = detect_format(&req);
    respond(fmt, ipv4, ipv6).await
}

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
            // (input, expected_ipv4, expected_ipv6, description)
            ("1.2.3.4", "1.2.3.4", "", "IPv4 address"),
            ("::1", "", "::1", "IPv6 loopback"),
            ("2001:db8:85a3::8a2e:370:7334", "", "2001:db8:85a3::8a2e:370:7334", "Full IPv6 address"),
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
            // (accept_header, expected_format, description)
            (None, Format::Text, "defaults to text when no header"),
            (Some("application/json"), Format::Json, "detects JSON from accept header"),
            (Some("text/html,application/json,*/*"), Format::Json, "finds JSON in mixed accept headers"),
            (Some("application/xml"), Format::Xml, "detects XML application type"),
            (Some("text/xml"), Format::Xml, "detects XML text type"),
            (Some("APPLICATION/JSON"), Format::Json, "handles case insensitive headers"),
            (Some("application/xml,application/json"), Format::Json, "JSON has priority over XML"),
            (Some("text/html,image/png"), Format::Text, "fallback to text for unrecognized types"),
            (Some("text/plain"), Format::Text, "text/plain returns text format"),
            (Some("application/pdf"), Format::Text, "unknown application type returns text"),
            (Some(""), Format::Text, "empty string returns text format"),
            (Some("application/json; charset=utf-8"), Format::Json, "JSON with charset parameter"),
            (Some("application/xml; charset=utf-8"), Format::Xml, "XML with charset parameter"),
        ];

        for (accept_header, expected_format, description) in test_cases {
            let actual_format = detect_format_from_accept(accept_header);
            assert_eq!(
                actual_format, expected_format,
                "Failed test case: {}", description
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
            ("192.168.1.1<script>&alert('xss')</script>", 
             "192.168.1.1&lt;script&gt;&amp;alert(&apos;xss&apos;)&lt;/script&gt;", 
             "complex XSS attempt escaped"),
        ];

        for (input, expected, description) in test_cases {
            assert_eq!(escape_xml(input), expected, "Failed: {}", description);
        }
    }

    #[test]
    fn xml_response_formatting() {
        let test_cases = vec![
            ("192.168.1.1", "2001:db8::1", 
             "<ip><ipv4>192.168.1.1</ipv4><ipv6>2001:db8::1</ipv6></ip>",
             "both IPv4 and IPv6"),
            ("", "", 
             "<ip><ipv4></ipv4><ipv6></ipv6></ip>",
             "empty IPs"),
            ("10.0.0.1", "", 
             "<ip><ipv4>10.0.0.1</ipv4><ipv6></ipv6></ip>",
             "IPv4 only"),
            ("", "fe80::1", 
             "<ip><ipv4></ipv4><ipv6>fe80::1</ipv6></ip>",
             "IPv6 only"),
        ];

        for (ipv4, ipv6, expected, description) in test_cases {
            let ipv4_escaped = escape_xml(ipv4);
            let ipv6_escaped = escape_xml(ipv6);
            let actual = format!("<ip><ipv4>{}</ipv4><ipv6>{}</ipv6></ip>", ipv4_escaped, ipv6_escaped);
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
            assert!(json.contains(&format!("\"ipv4\":\"{}\"", ipv4)), 
                    "IPv4 missing for: {}", description);
            assert!(json.contains(&format!("\"ipv6\":\"{}\"", ipv6)), 
                    "IPv6 missing for: {}", description);
        }
    }
}
