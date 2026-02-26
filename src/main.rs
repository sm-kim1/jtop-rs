mod app;
mod event;
mod hardware;
mod remote;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "jtop-rs", about = "Rust TUI for NVIDIA Jetson monitoring")]
pub struct Args {
    /// Remote Jetson host (e.g., 192.168.100.54)
    #[arg(short = 'H', long)]
    pub host: Option<String>,

    /// SSH username
    #[arg(short, long, default_value = "aceworks")]
    pub user: String,

    /// SSH port
    #[arg(short, long, default_value_t = 22)]
    pub port: u16,

    /// SSH key path
    #[arg(short, long)]
    pub key: Option<String>,

    /// Refresh interval in milliseconds
    #[arg(short, long, default_value_t = 500)]
    pub interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter("jtop_rs=info")
        .with_target(false)
        .init();

    tracing::info!("jtop-rs starting with args: {:?}", args);

    app::run(args).await
}
