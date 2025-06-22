# whatismyip

A Cloudflare Worker that returns your public IP address and automatically updates DNS records.

## What It Does

This worker provides three main features:

1. **Returns your IP address** in different formats (text, JSON, or XML)
2. **Stores IP history** using Cloudflare KV storage
3. **Updates DNS records** automatically when your IP changes

## Quick Start

### 1. Get Your IP Address

```bash
# Plain text (default) - shows IPv4 on first line, IPv6 on second line
curl "https://your-worker.workers.dev?homename=myhome"

# JSON format
curl -H "Accept: application/json" "https://your-worker.workers.dev?homename=myhome"

# XML format  
curl -H "Accept: application/xml" "https://your-worker.workers.dev?homename=myhome"
```

The `homename` parameter is required and can contain letters, numbers, `-`, `_`, or `.`

### 2. Response Formats

**Plain Text (default):**
```
192.168.1.100
2001:db8::1
```

**JSON:**
```json
{"ipv4": "192.168.1.100", "ipv6": "2001:db8::1"}
```

**XML:**
```xml
<ip><ipv4>192.168.1.100</ipv4><ipv6>2001:db8::1</ipv6></ip>
```

## Setup & Deployment

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (for wrangler)
- Cloudflare account

### Step 1: Install Tools

```bash
# Install Rust worker build tool
cargo install worker-build

# Login to Cloudflare
npx wrangler login
```

### Step 2: Configure Your Worker

Edit `wrangler.toml` and replace the placeholder values:

```toml
name = "your-worker-name"  # Choose your worker name
main = "build/worker/shim.mjs"
compatibility_date = "2023-12-01"

[build]
command = "cargo install -q worker-build && worker-build --release"

# Replace with your actual KV namespace ID (see Step 3)
[[kv_namespaces]]
binding = "IP_STORE"
id = "your-actual-kv-namespace-id"
preview_id = "your-actual-preview-kv-namespace-id"

[vars]
CF_ZONE_ID = "your-actual-zone-id"     # Your Cloudflare Zone ID
CF_DOMAIN = "yourdomain.com"           # Your domain name
```

### Step 3: Create KV Namespace

```bash
# Create KV storage for IP addresses and DNS record IDs
npx wrangler kv namespace create IP_STORE
npx wrangler kv namespace create IP_STORE --preview
```

Copy the namespace IDs from the output into your `wrangler.toml` file.

### Step 4: Set Secrets

```bash
# Required: Cloudflare API token for DNS updates
npx wrangler secret put CF_API_TOKEN

# Optional: API token for request authentication (leave empty to disable)
npx wrangler secret put API_TOKEN
```

**To get your Cloudflare API token:**
1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com/profile/api-tokens)
2. Create token with permissions: `Zone:Zone:Read`, `Zone:DNS:Edit`
3. Include your specific zone in the token scope

### Step 5: Deploy

```bash
# Deploy your worker
npx wrangler deploy
```

Your worker will be available at: `https://your-worker-name.your-subdomain.workers.dev`

## Configuration Options

### Environment Variables (Secrets)

Set these using `npx wrangler secret put <NAME>`:

- **`CF_API_TOKEN`** (required): Cloudflare API token for DNS operations
- **`API_TOKEN`** (optional): If set, requires `Authorization: Bearer <token>` header on all requests

### Variables in wrangler.toml

- **`CF_ZONE_ID`**: Your Cloudflare Zone ID (found in domain overview)
- **`CF_DOMAIN`**: Your domain name (e.g., `example.com`)

## How DNS Updates Work

1. **First request**: Worker checks if DNS records exist for `homename.yourdomain.com`
2. **IP change detected**: Worker automatically creates or updates A/AAAA records
3. **Record IDs stored**: Worker remembers DNS record IDs in KV storage for efficiency
4. **Future requests**: Worker uses stored IDs to update records quickly

**Example**: If `homename=home` and `CF_DOMAIN=example.com`, the worker manages:
- `home.example.com` A record (IPv4)
- `home.example.com` AAAA record (IPv6)

## Security

### Authentication (Optional)

If you set the `API_TOKEN` secret, all requests must include:

```bash
curl -H "Authorization: Bearer your-secret-token" \
     "https://your-worker.workers.dev?homename=myhome"
```

### Safe Configuration

- Secrets are stored securely in Cloudflare Workers
- KV namespace IDs in `wrangler.toml` are safe to commit
- No sensitive data is exposed in the repository

## Development

### Local Testing

```bash
# Run locally (uses preview KV namespace)
npx wrangler dev
```

Test locally at: `http://localhost:8787?homename=test`

### Running Tests

```bash
# Run Rust unit tests
cargo test
```

## Production Setup

For production, you can create a separate configuration:

1. Copy `wrangler.production.toml.example` to `wrangler.production.toml`
2. Configure production-specific values
3. Deploy with: `npx wrangler deploy --config wrangler.production.toml --env production`

## Custom Domain (Optional)

1. Add your domain to Cloudflare
2. In Cloudflare dashboard: Workers & Pages → Your Worker → Settings → Triggers
3. Add custom domain

## Project Structure

```
whatismyip/
├── src/
│   ├── lib.rs          # Main worker entry point
│   ├── auth.rs         # Authentication logic
│   ├── config.rs       # Configuration management
│   ├── dns.rs          # DNS record management
│   ├── ip.rs           # IP address handling
│   ├── request.rs      # Request parsing and validation
│   ├── response.rs     # Response formatting
│   └── service.rs      # Core business logic
├── wrangler.toml       # Worker configuration
├── wrangler.production.toml.example  # Production template
└── Cargo.toml          # Rust dependencies
```

## Troubleshooting

### Common Issues

**"homename parameter required"**
- Add `?homename=yourname` to your URL

**"Unauthorized"**
- Either remove `API_TOKEN` secret or add `Authorization: Bearer <token>` header

**DNS updates not working**
- Check `CF_API_TOKEN` has correct permissions
- Verify `CF_ZONE_ID` and `CF_DOMAIN` are correct
- Check Cloudflare dashboard for DNS records

**KV errors**
- Ensure KV namespace is created and ID is correct in `wrangler.toml`

### Getting Help

Check the worker logs:
```bash
npx wrangler tail
```
