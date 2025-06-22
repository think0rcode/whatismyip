use worker::*;
use serde::Serialize;
use subtle::ConstantTimeEq;

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

async fn respond(format: Format, ipv4: String, ipv6: String) -> Result<Response> {
    match format {
        Format::Text => Response::ok(text_body(&ipv4, &ipv6)),
        Format::Json => Response::from_json(&IpPayload { ipv4, ipv6 }),
        Format::Xml => {
            let ipv4_escaped = ipv4.replace('&', "&amp;").replace('<', "&lt;");
            let ipv6_escaped = ipv6.replace('&', "&amp;").replace('<', "&lt;");
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
        let (v4, v6) = split_ip("1.2.3.4");
        assert_eq!(v4, "1.2.3.4");
        assert_eq!(v6, "");
        let (v4, v6) = split_ip("::1");
        assert_eq!(v4, "");
        assert_eq!(v6, "::1");
    }
}
