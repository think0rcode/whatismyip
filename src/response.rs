use serde::Serialize;
use worker::*;
use crate::request::Format;

// Constants
const HEADER_CONTENT_TYPE: &str = "Content-Type";
const CONTENT_TYPE_XML: &str = "application/xml";

/// Represents the IP address payload returned by the API
#[derive(Serialize)]
pub struct IpPayload {
    pub ipv4: String,
    pub ipv6: String,
}

/// Response formatting utilities
pub struct ResponseUtils;

impl ResponseUtils {
    /// Creates a response in the specified format
    pub async fn create_response(format: Format, ipv4: String, ipv6: String) -> Result<Response> {
        match format {
            Format::Text => Response::ok(Self::format_text(&ipv4, &ipv6)),
            Format::Json => Response::from_json(&IpPayload { ipv4, ipv6 }),
            Format::Xml => Self::create_xml_response(&ipv4, &ipv6),
        }
    }

    /// Formats IP addresses as plain text
    pub fn format_text(ipv4: &str, ipv6: &str) -> String {
        format!("{}\n{}\n", ipv4, ipv6)
    }

    /// Creates an XML response with proper escaping
    fn create_xml_response(ipv4: &str, ipv6: &str) -> Result<Response> {
        let ipv4_escaped = Self::escape_xml(ipv4);
        let ipv6_escaped = Self::escape_xml(ipv6);
        let body = format!(
            "<ip><ipv4>{}</ipv4><ipv6>{}</ipv6></ip>",
            ipv4_escaped, ipv6_escaped
        );
        let mut resp = Response::ok(body)?;
        resp.headers_mut().set(HEADER_CONTENT_TYPE, CONTENT_TYPE_XML)?;
        Ok(resp)
    }

    /// Escapes XML special characters to prevent injection
    pub fn escape_xml(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_formatting() {
        assert_eq!(ResponseUtils::format_text("1.1.1.1", ""), "1.1.1.1\n\n");
        assert_eq!(ResponseUtils::format_text("", "::1"), "\n::1\n");
        assert_eq!(ResponseUtils::format_text("1.1.1.1", "::1"), "1.1.1.1\n::1\n");
        assert_eq!(ResponseUtils::format_text("", ""), "\n\n");
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
            assert_eq!(ResponseUtils::escape_xml(input), expected, "Failed: {}", description);
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
            let ipv4_escaped = ResponseUtils::escape_xml(ipv4);
            let ipv6_escaped = ResponseUtils::escape_xml(ipv6);
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
} 
