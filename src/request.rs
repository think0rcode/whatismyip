use worker::*;

// Constants
const HEADER_CF_CONNECTING_IP: &str = "CF-Connecting-IP";
const HEADER_ACCEPT: &str = "Accept";
const PARAM_HOMENAME: &str = "homename";

/// Supported response formats
#[derive(Debug, PartialEq)]
pub enum Format {
    Text,
    Json,
    Xml,
}

/// Request context containing parsed and validated request data
pub struct RequestContext {
    /// Validated hostname for DNS record management
    pub homename: String,
    /// Client IP address from Cloudflare headers
    pub client_ip: String,
    /// Desired response format (text, JSON, or XML)
    pub format: Format,
}

impl RequestContext {
    /// Parse request context from incoming request
    pub fn from_request(req: &Request) -> Result<Self> {
        let url = req.url()?;
        let homename = Self::extract_homename(&url)?;
        let client_ip = Self::extract_client_ip(req)?;
        let format = Self::detect_format(req);

        Ok(Self {
            homename,
            client_ip,
            format,
        })
    }

    /// Extract and validate homename from URL query parameters
    fn extract_homename(url: &Url) -> Result<String> {
        let homename = url.query_pairs()
            .find(|(k, _)| k == PARAM_HOMENAME)
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| Error::RustError("homename parameter required".to_string()))?;

        if !Self::is_valid_homename(&homename) {
            return Err(Error::RustError("invalid homename".to_string()));
        }

        Ok(homename)
    }

    /// Extract client IP from Cloudflare headers
    fn extract_client_ip(req: &Request) -> Result<String> {
        Ok(req.headers()
            .get(HEADER_CF_CONNECTING_IP)?
            .unwrap_or_default())
    }

    /// Detects the desired response format from the request
    fn detect_format(req: &Request) -> Format {
        let accept_header = req.headers()
            .get(HEADER_ACCEPT)
            .ok()
            .flatten();
        Self::detect_format_from_accept(accept_header.as_deref())
    }

    /// Detects the desired response format from the Accept header
    pub fn detect_format_from_accept(accept_header: Option<&str>) -> Format {
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

    /// Validates that the homename only contains ASCII letters, '-' or '_'
    pub fn is_valid_homename(name: &str) -> bool {
        !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphabetic() || c == '-' || c == '_')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let actual_format = RequestContext::detect_format_from_accept(accept_header);
            assert_eq!(
                actual_format, expected_format,
                "Failed test case: {}",
                description
            );
        }
    }

    #[test]
    fn homename_validation() {
        let test_cases = vec![
            ("valid", true, "simple valid name"),
            ("valid-name", true, "name with hyphen"),
            ("valid_name", true, "name with underscore"),
            ("ValidName", true, "name with capitals"),
            ("valid123", false, "name with numbers - should be invalid"),
            ("", false, "empty name"),
            ("invalid.name", false, "name with dot"),
            ("invalid name", false, "name with space"),
            ("invalid@name", false, "name with special char"),
            ("a", true, "single character"),
            ("A", true, "single capital"),
            ("-", true, "single hyphen"),
            ("_", true, "single underscore"),
            ("valid-name_test", true, "complex valid name"),
            ("123invalid", false, "starts with number"),
            ("invalid!", false, "ends with special char"),
        ];

        for (input, expected, description) in test_cases {
            let result = RequestContext::is_valid_homename(input);
            assert_eq!(result, expected, "Failed: {}", description);
        }
    }
} 
