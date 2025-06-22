# whatismyip

A Cloudflare Worker written in Rust that returns your public IP address in one of three formats depending on the request:

1. **Plain text** – always returns two lines: the IPv4 address on the first line and the IPv6 address on the second. Missing values are left blank.
2. **JSON** – returns `{ "ipv4": "...", "ipv6": "..." }`.
3. **XML** – returns `<ip><ipv4>...</ipv4><ipv6>...</ipv6></ip>`.

The response type is chosen based on the request's `Content-Type` header. If that header is missing,
the optional `format` query parameter or the `Accept` header is used as a fallback.

### Authentication

If the `API_TOKEN` environment variable is provided at compile time, the worker
expects an `Authorization: Bearer <token>` header on all requests.

## Tests

Run unit tests with:

```bash
cargo test
```

This crate is configured for deployment with `wrangler` or any other Cloudflare Worker toolchain that consumes a `cdylib` WebAssembly target.
