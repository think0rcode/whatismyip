use worker::*;
use serde::Serialize;

const API_TOKEN: Option<&str> = option_env!("API_TOKEN");

#[derive(Serialize)]
struct IpPayload {
    ipv4: String,
    ipv6: String,
}

enum Format {
    Text,
    Json,
    Xml,
}

fn detect_format(req: &Request) -> Format {
    if let Ok(Some(ct)) = req.headers().get("Content-Type") {
        let ct = ct.to_lowercase();
        if ct.contains("application/json") {
            return Format::Json;
        }
        if ct.contains("application/xml") || ct.contains("text/xml") {
            return Format::Xml;
        }
        if ct.contains("text/plain") {
            return Format::Text;
        }
    }
    if let Ok(url) = req.url() {
        if let Some((_, v)) = url.query_pairs().find(|(k, _)| k == "format") {
            match v.as_ref().to_lowercase().as_str() {
                "json" => return Format::Json,
                "xml" => return Format::Xml,
                "text" => return Format::Text,
                _ => {}
            }
        }
    }
    if let Ok(Some(accept)) = req.headers().get("Accept") {
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
    format!("{}\n{}", ipv4, ipv6)
}

fn check_auth(req: &Request) -> bool {
    if let Some(token) = API_TOKEN {
        let expected = format!("Bearer {}", token);
        match req.headers().get("Authorization").ok().flatten() {
            Some(ref h) if h == &expected => true,
            _ => false,
        }
    } else {
        true
    }
}

async fn respond(format: Format, ipv4: String, ipv6: String) -> Result<Response> {
    match format {
        Format::Text => Response::ok(text_body(&ipv4, &ipv6)),
        Format::Json => Response::from_json(&IpPayload { ipv4, ipv6 }),
        Format::Xml => {
            let mut resp = Response::ok(format!("<ip><ipv4>{}</ipv4><ipv6>{}</ipv6></ip>", ipv4, ipv6))?;
            resp.headers_mut().set("Content-Type", "application/xml")?;
            Ok(resp)
        }
    }
}

pub async fn handler(req: Request) -> Result<Response> {
    if !check_auth(&req) {
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
pub async fn main(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    handler(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_formatting() {
        assert_eq!(text_body("1.1.1.1", ""), "1.1.1.1\n");
        assert_eq!(text_body("", "::1"), "\n::1");
        assert_eq!(text_body("1.1.1.1", "::1"), "1.1.1.1\n::1");
        assert_eq!(text_body("", ""), "\n");
    }

    #[test]
    fn ip_splitting() {
        let (v4, v6) = split_ip("1.2.3.4");
        assert_eq!(v4, "1.2.3.4");
        assert_eq!(v6, "");
        let (v4, v6) = split_ip("::1");
        assert_eq!(v4, "");
        assert_eq!(v6, "::1");
    }
}
