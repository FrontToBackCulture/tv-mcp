# tv-mcp

Rust MCP (Model Context Protocol) server that connects Claude Code to ThinkVAL's business operations. Exposes 100+ tools for CRM, projects, email, VAL domain sync, content generation, and more. Runs as a stdio server (Claude Desktop/Code) or HTTP server (testing).

## Tech Stack

- **Language:** Rust (Edition 2021)
- **Async:** Tokio (runtime, stdio, networking)
- **HTTP Server:** Axum (for `--http` mode)
- **HTTP Client:** Reqwest (120s timeout, TLS)
- **Serialization:** Serde + serde_json (preserve_order)
- **AWS:** S3 + SES SDKs
- **Markdown:** pulldown-cmark (MD→HTML), css-inline (email styling)
- **Testing:** Wiremock (HTTP mocking)

## Architecture

```
src/
├── main.rs                 # Entry point: --version, --http, or stdio
├── lib.rs                  # Library root
├── core/                   # Foundation
│   ├── error.rs            # CommandError enum (Network, Http, Parse, NotFound, Config, Io, Internal)
│   ├── settings.rs         # ~/.tv-mcp/settings.json management (30+ keys)
│   └── supabase.rs         # Supabase REST client + bot JWT minting
├── server/                 # MCP protocol layer
│   ├── server.rs           # HTTP (Axum) + stdio server implementations
│   ├── protocol.rs         # JSON-RPC 2.0 types, Tool/ToolResult definitions
│   └── tools/              # Tool registry & dispatching
│       ├── mod.rs           # list_tools() + call_tool() dispatcher
│       ├── work.rs          # Projects, tasks, initiatives, milestones, skills, users
│       ├── crm.rs           # Companies, contacts, activities
│       ├── email.rs         # Campaigns, groups, send, drafts
│       ├── generate.rs      # Gamma presentations, Nanobanana images
│       ├── val_sync.rs      # VAL domain sync operations
│       ├── feed.rs          # Feed cards
│       ├── blog.rs          # Blog articles
│       ├── discussions.rs   # Comments on entities
│       ├── notifications.rs # Mention notifications
│       ├── intercom.rs      # Help center publishing
│       ├── docgen.rs        # PDF generation (proposals, order forms)
│       ├── apollo.rs        # Apollo prospect search
│       └── whatsapp.rs      # WhatsApp summaries
└── modules/                # Business logic (separated from server layer)
    ├── apollo/              # Apollo API integration
    ├── blog/                # Blog CRUD
    ├── crm/                 # Companies, contacts, activities
    ├── discussions/         # Entity comments
    ├── email/               # Campaigns + transactional (AWS SES)
    ├── feed/                # Feed cards
    ├── notifications/       # Mentions
    ├── tools/               # Gamma API, Nanobanana (Gemini), Intercom, DocGen
    ├── val_sync/            # VAL domain sync (databases, workflows, dashboards, queries, fields, monitoring)
    ├── whatsapp/            # Chat summaries
    └── work/                # Projects, tasks, milestones, initiatives, labels, skills, sessions, users
```

## Commands

```bash
cargo build              # Debug build (HTTP port 23817)
cargo build --release    # Release build (HTTP port 23816, LTO thin)
cargo test               # Run tests (wiremock-based)
```

After building:
```bash
# Symlink for Claude Code (debug only — release build hangs due to Tauri AppKit linking)
ln -sf "$(pwd)/target/debug/tv-mcp" ~/.tv-desktop/bin/tv-mcp
pkill -9 tv-mcp          # Kill running processes so Claude Code picks up new binary
```

## Running Modes

```bash
# Stdio mode (default) — Claude Desktop/Code connects via stdin/stdout
./tv-mcp

# HTTP mode — for testing
./tv-mcp --http          # Listens on 127.0.0.1:23817 (debug) or :23816 (release)

# Version check
./tv-mcp --version
```

## Configuration

### Settings File

Primary: `~/.tv-mcp/settings.json` (fallback: `~/.tv-desktop/settings.json`)

Key settings:
- `supabase_url`, `supabase_anon_key` — Workspace Supabase credentials
- `gamma_api_key`, `gemini_api_key`, `anthropic_api_key`, `apollo_api_key`, `intercom_api_key`, `notion_api_key` — API keys
- `aws_access_key_id`, `aws_secret_access_key` — AWS for S3/SES
- `val_email_{domain}`, `val_password_{domain}` — Per-domain VAL API credentials
- `ws:{workspace_id}:{key}` — Workspace-scoped settings (auto-fallback to global)

### Environment Variables

- `TV_BOT_API_KEY` — Bot authentication. If set, mints JWT from gateway `bot-token` edge function.
- `RUST_LOG` — Logging level (env_logger)

## Supabase Client

`core/supabase.rs` implements a REST client over Supabase's PostgREST API:

- `select<T>(table, query)` — GET with query params (e.g., `name=eq.value`, `order=updated_at.desc`)
- `select_single<T>(table, query)` — Returns first row or None
- `insert<T, R>(table, body)` — POST with `Prefer: return=representation`
- `update<T, R>(table, query, body)` — PATCH with filter
- `upsert_on<T, R>(table, body, conflict_cols)` — INSERT/UPDATE on conflict
- `delete(table, query)` — DELETE with filter
- `rpc<T, R>(function, body)` — Call Postgres functions

### Auth Flow

1. On startup, reads `TV_BOT_API_KEY` from env
2. If set, calls gateway `POST /functions/v1/bot-token` with the API key
3. Gateway validates key hash → looks up bot identity → mints workspace JWT
4. All subsequent requests use JWT in `Authorization: Bearer` header
5. JWT cached globally, auto-refreshed 5 min before expiry

## MCP Protocol

Implements JSON-RPC 2.0 with three methods:
- `initialize` — Returns server info + capabilities
- `tools/list` — Returns all tool definitions (name, description, input_schema)
- `tools/call` — Dispatches to tool handler by name

### Tool Dispatch Pattern

```rust
// tools/mod.rs aggregates tools from all modules
pub fn list_tools() -> Vec<Tool> {
    let mut tools = vec![];
    tools.extend(work::tools());
    tools.extend(crm::tools());
    // ... etc
    tools
}

pub async fn call_tool(name: &str, args: Value) -> ToolResult {
    // Route by prefix/name to module handlers
}
```

Each tool module (e.g., `work.rs`) has:
- `tools() -> Vec<Tool>` — Tool definitions with JSON Schema
- `call(name, args) -> ToolResult` — Handler dispatch

## Special Features

### Binary Auto-Update

After each stdio request, checks if the executable's mtime changed. If so, logs and exits — Claude Code auto-restarts with the new binary. This means you just rebuild and `pkill`; no manual restart needed.

### Orphan Detection

Monitors parent PID every 10s. If parent dies (or becomes PID 1 on Unix), exits cleanly. Prevents stale processes when Claude Code terminates.

### Workspace Multi-Tenancy

Task-local `WORKSPACE_OVERRIDE` pins a workspace ID per request. Workspace-scoped settings use `ws:{id}:{key}` format with automatic fallback to global keys.

## Gotchas

### Cargo stale builds — touch before building

Cargo uses file timestamps to detect changes. Claude Code's Edit tool sometimes doesn't update mtime. **Always touch edited files before building:**

```bash
touch src/the_file_you_edited.rs
cargo build
```

If `cargo build` says "Finished" without "Compiling", it missed the change.

### Release build hangs — use debug for standalone binary

`cargo build --release` with the full dependency tree (includes Tauri/AppKit/WebKit via shared code path) can hang. Use `cargo build` (debug) for the standalone `tv-mcp` binary.

### Symlink, don't copy

Use `ln -sf` to symlink the binary to `~/.tv-desktop/bin/tv-mcp`. Copying triggers macOS quarantine (Gatekeeper), which blocks execution.

### Kill after rebuild

Running tv-mcp processes must be killed after rebuilding. Claude Code auto-restarts on the next tool call, but won't pick up the new binary while the old process is alive:

```bash
pkill -9 tv-mcp
```

### Settings migration

`load_settings()` includes automatic Supabase URL migration from Mumbai to Singapore region. Don't remove this — it handles legacy settings files.
