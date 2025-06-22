use std::net::IpAddr;

/// IP address utilities
pub struct IpUtils;

impl IpUtils {
    /// Splits an IP address string into IPv4 and IPv6 components
    pub fn split_ip(ip: &str) -> (String, String) {
        match ip.parse::<IpAddr>() {
            Ok(IpAddr::V4(ipv4)) => (ipv4.to_string(), String::new()),
            Ok(IpAddr::V6(ipv6)) => (String::new(), ipv6.to_string()),
            Err(_) => (String::new(), String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let (actual_v4, actual_v6) = IpUtils::split_ip(input);
            assert_eq!(actual_v4, expected_v4, "IPv4 failed for: {}", description);
            assert_eq!(actual_v6, expected_v6, "IPv6 failed for: {}", description);
        }
    }
}
