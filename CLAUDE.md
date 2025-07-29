# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Cloudflare Worker written in Rust that provides IP address services and automatic DNS record management. The worker returns public IP addresses in multiple formats (text, JSON, XML) and automatically updates Cloudflare DNS records when IP addresses change.

## Architecture

The codebase follows a modular architecture with clear separation of concerns:

- **`src/lib.rs`**: Main entry point and request handler orchestration
- **`src/auth.rs`**: Authentication logic for optional API token validation
- **`src/config.rs`**: Environment variable configuration management
- **`src/dns.rs`**: DNS record operations via Cloudflare API
- **`src/ip.rs`**: IP address parsing and validation utilities
- **`src/request.rs`**: HTTP request parsing and context extraction
- **`src/response.rs`**: Response formatting (text, JSON, XML)
- **`src/service.rs`**: Core business logic for DNS updates

The application uses Cloudflare KV storage to persist IP address history and DNS record IDs for efficient updates.

## Development Commands

### Building and Testing
```bash
# Install required build tool
cargo install worker-build

# Build the worker (development)
worker-build

# Build for production (release mode)
worker-build --release

# Run Rust unit tests
cargo test
```

### Local Development
```bash
# Run worker locally with hot reload
npx wrangler dev

# Test locally
curl "http://localhost:8787?homename=test"
```

### Deployment
```bash
# Deploy to Cloudflare Workers
npx wrangler deploy

# Deploy production environment
npx wrangler deploy --config wrangler.production.toml --env production

# Check worker logs
npx wrangler tail
```

### Configuration Management
```bash
# Create KV namespace
npx wrangler kv namespace create IP_STORE
npx wrangler kv namespace create IP_STORE --preview

# Set secrets
npx wrangler secret put CF_API_TOKEN
npx wrangler secret put API_TOKEN  # Optional for auth
```

## Key Configuration

The worker requires several environment variables configured in `wrangler.toml`:

- **KV_NAMESPACES**: `IP_STORE` binding for persistent storage
- **CF_ZONE_ID**: Cloudflare zone ID for DNS operations
- **CF_DOMAIN**: Base domain for DNS record creation
- **CF_API_TOKEN**: Secret for Cloudflare API authentication
- **API_TOKEN**: Optional secret for request authentication

## DNS Record Management

The worker automatically manages DNS records using this pattern:
- Request with `?homename=mydevice` creates/updates `mydevice.yourdomain.com`
- Supports both IPv4 (A records) and IPv6 (AAAA records)
- Record IDs are cached in KV storage for efficient updates
- Only updates DNS when IP addresses actually change

## Response Formats

The worker supports three response formats based on `Accept` header:
- **text/plain** (default): IPv4 and IPv6 on separate lines
- **application/json**: `{"ipv4": "...", "ipv6": "..."}`
- **application/xml**: `<ip><ipv4>...</ipv4><ipv6>...</ipv6></ip>`