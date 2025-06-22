# whatismyip

A Cloudflare Worker written in Rust that returns your public IP address in one of three formats depending on the request:

1. **Plain text** – always returns two lines: the IPv4 address on the first line and the IPv6 address on the second. Missing values are left blank.
2. **JSON** – returns `{ "ipv4": "...", "ipv6": "..." }`.
3. **XML** – returns `<ip><ipv4>...</ipv4><ipv6>...</ipv6></ip>`.

The response type is chosen based on the request's `Accept` header. If that header is missing,
the default format will be Plain text.

## Usage Examples

```bash
# Get IP in plain text format (default)
curl https://your-worker.your-subdomain.workers.dev

# Get IP in JSON format
curl -H "Accept: application/json" https://your-worker.your-subdomain.workers.dev

# Get IP in XML format
curl -H "Accept: application/xml" https://your-worker.your-subdomain.workers.dev
```

## Authentication

If the `API_TOKEN` secret is set in the worker's environment, the worker
expects an `Authorization: Bearer <token>` header on all requests. The token
comparison is performed using constant-time equality via the `subtle` crate to
help avoid timing attacks.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (for npx)

### Running Tests

Run unit tests with:

```bash
cargo test
```

### Local Development

To run the worker locally for development:

```bash
npx wrangler dev
```

## Deployment

### Deploy to Cloudflare Workers

1. **Install worker-build for Rust compilation:**
   ```bash
   cargo install worker-build
   ```

2. **Login to Cloudflare:**
   ```bash
   npx wrangler login
   ```

3. **Configure your worker name (optional):**
   Edit `wrangler.toml` and change the `name` field to your preferred worker name.

4. **Set up authentication (optional):**
   If you want to require API token authentication:
   ```bash
   # Set the API_TOKEN secret
   npx wrangler secret put API_TOKEN
   # Enter your desired token when prompted
   ```

5. **Deploy:**
   ```bash
   npx wrangler deploy
   ```

   Your worker will be available at `https://your-worker-name.your-subdomain.workers.dev`

### Environment Variables

- `API_TOKEN` (optional): If set, requires Bearer token authentication for all requests

### Custom Domain (Optional)

To use a custom domain:

1. Add your domain to Cloudflare
2. In the Cloudflare dashboard, go to Workers & Pages
3. Select your worker
4. Go to Settings > Triggers
5. Add a custom domain

## Project Structure

```
whatismyip/
├── src/
│   └── lib.rs          # Main worker logic
├── Cargo.toml          # Rust dependencies
├── wrangler.toml       # Cloudflare Workers configuration
└── README.md           # This file
```

## License

This project is configured for deployment with `wrangler` and any other Cloudflare Worker toolchain that consumes a `cdylib` WebAssembly target.
