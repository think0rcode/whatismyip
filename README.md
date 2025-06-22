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
curl "https://your-worker.your-subdomain.workers.dev?homename=myhome"

# Get IP in JSON format
curl -H "Accept: application/json" https://your-worker.your-subdomain.workers.dev

# Get IP in XML format
curl -H "Accept: application/xml" https://your-worker.your-subdomain.workers.dev
```

Each request requires a `homename` query parameter containing only letters,
numbers, `-`, `_`, or `.`. The worker stores the last seen IPs for that homename in KV and,
when configured, updates the associated Cloudflare DNS records if the address
changes.

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
# Make sure you have set up your .dev.vars file first
cp .dev.vars.example .dev.vars
# Edit .dev.vars with your actual values

# Run locally (uses preview KV namespace)
npx wrangler dev
```

## Deployment

### Security Note

This repository uses a secure configuration approach to prevent leaking sensitive KV namespace IDs:

- `wrangler.toml` - Contains placeholder values safe for public repositories
- `wrangler.production.toml.example` - Template for production configuration
- `wrangler.production.toml` - Your actual production config (git-ignored)
- `.env` - Your environment variables (git-ignored)

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

4. **Set up environment variables:**
   ```bash
   # For local development, copy and edit the .dev.vars file
   cp .dev.vars.example .dev.vars
   # Edit .dev.vars with your actual values
   
   # For production deployment, you have two options:
   
   # Option 1: Use wrangler secrets (recommended for sensitive data)
   npx wrangler secret put CF_API_TOKEN
   npx wrangler secret put API_TOKEN  # optional
   
   # Option 2: Set values directly in wrangler.production.toml (not recommended for secrets)
   # Edit wrangler.production.toml and uncomment/set the [env.production.vars] values
   ```

5. **Create KV namespace and production config:**
   ```bash
   # Create the required KV namespace for storing IP addresses and DNS record IDs
   npx wrangler kv namespace create PROD_KV
   
   # Create production configuration file (not committed to git)
   cp wrangler.production.toml.example wrangler.production.toml
   ```
   
   After creating the namespace, you'll see output like:
   ```
   [[kv_namespaces]]
   binding = "PROD_KV"
   id = "your-actual-namespace-id-here"
   ```
   
   **Important**: Update your `wrangler.production.toml` file with the actual namespace ID from the output above. This file is git-ignored to keep your KV namespace IDs private. The KV namespace should be configured under the production environment section as `[[env.production.kv_namespaces]]`.

6. **Set up authentication (optional):**
   If you want to require API token authentication, make sure the `API_TOKEN` 
   variable is set in your `.env` file. This was already covered in step 4.

7. **Deploy:**
   ```bash
   # Deploy to production using your private production config
   npx wrangler deploy --config wrangler.production.toml --env production
   
   # Or deploy to default environment for testing
   npx wrangler deploy
   ```

   Your worker will be available at `https://your-worker-name.your-subdomain.workers.dev`
   
   **Environment Management:**
   - `npx wrangler deploy` - Uses default configuration (safe for public repo)
   - `npx wrangler deploy --config wrangler.production.toml --env production` - Uses private production config
   - `npx wrangler dev` - Local development with preview KV namespace

### Environment Variables

The worker uses the following environment variables:

- `CF_ZONE_ID`: Cloudflare Zone ID used for DNS updates
- `CF_DOMAIN`: The domain name to use for DNS records
- `CF_API_TOKEN`: Cloudflare API token with permission to edit DNS records
- `API_TOKEN` (optional): If set, requires Bearer token authentication for all requests

**Configuration:**

**For Local Development:**
- Copy `.dev.vars.example` to `.dev.vars` and fill in your values
- The `.dev.vars` file is used by `wrangler dev` for local development
- This file is ignored by git to keep your configuration private

**For Production Deployment:**
- Use `wrangler secret put` for sensitive values like API tokens (recommended)
- Or set values directly in your `wrangler.production.toml` file for non-sensitive configuration

### Automatic DNS Record Management

The worker now automatically manages DNS record IDs without requiring manual setup:

1. **First Request**: When a `homename` is used for the first time, the worker will:
   - Check KV storage for existing DNS record IDs
   - If not found, query Cloudflare to find existing DNS records for `homename.CF_DOMAIN`
   - If no records exist, create new DNS records when IP addresses are updated
   - Store the record IDs in KV for future use

2. **Subsequent Requests**: The worker uses the stored record IDs to update DNS records efficiently

**Note**: The worker requires a KV namespace to store IP addresses and DNS record IDs. The KV binding name is hardcoded in the source code and must match the binding name in your wrangler configuration. No manual DNS record ID setup is required - the worker handles this automatically.

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
│   ├── lib.rs          # Main worker entry point and routing
│   ├── auth.rs         # Authentication logic and token validation
│   ├── config.rs       # Configuration management and environment variables
│   ├── dns.rs          # Cloudflare DNS record management
│   ├── ip.rs           # IP address extraction and validation
│   ├── request.rs      # Request handling and validation
│   ├── response.rs     # Response formatting (plain text, JSON, XML)
│   └── service.rs      # Core service logic and IP storage
├── Cargo.toml          # Rust dependencies and package configuration
├── Cargo.lock          # Dependency lock file
├── wrangler.toml       # Cloudflare Workers configuration (public)
├── wrangler.production.toml.example  # Production config template
├── .env.example        # Environment variables template
├── .gitignore          # Git ignore patterns
└── README.md           # This file
```

## License

This project is configured for deployment with `wrangler` and any other Cloudflare Worker toolchain that consumes a `cdylib` WebAssembly target.
