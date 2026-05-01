use tv_mcp::server;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_ok() {
        env_logger::init();
    }

    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("tv-mcp {}", VERSION);
        return;
    }

    if args.iter().any(|a| a == "--sync-tools") {
        match tv_mcp::modules::mcp_tools::sync_mcp_tools().await {
            Ok(r) => {
                println!("{}", serde_json::to_string(&r).unwrap_or_else(|_| "{}".to_string()));
            }
            Err(e) => {
                eprintln!("sync-mcp-tools failed: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    eprintln!("[tv-mcp] v{} starting", VERSION);

    if args.iter().any(|a| a == "--http") {
        eprintln!("[tv-mcp] HTTP mode on http://localhost:{}", server::server::DEFAULT_PORT);
        if let Err(e) = server::server::run_http(server::server::DEFAULT_PORT).await {
            eprintln!("[tv-mcp] server error: {}", e);
            std::process::exit(1);
        }
    } else {
        eprintln!("[tv-mcp] stdio mode");
        if let Err(e) = server::server::run_stdio().await {
            eprintln!("[tv-mcp] server error: {}", e);
            std::process::exit(1);
        }
    }
}
