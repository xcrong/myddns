mod cli;
mod config;
mod model;
mod utils;

use clap::Parser;
use cli::{Cli, handle_netcards_command, handle_test_command};
use config::load_config;
use model::CloudflareClient;
use std::error::Error;
use utils::{ensure_zone_id, get_local_ipv6_address, process_all_domains};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if cli.netcards {
        return handle_netcards_command();
    }

    if cli.test || cli.test_config_path.is_some() {
        return handle_test_command(cli.test_config_path);
    }

    let config_fpath = cli
        .config
        .unwrap_or_else(|| "/etc/myddns/config.toml".to_string());
    let config = load_config(&config_fpath)?;

    println!("=== Cloudflare IPv6 DDNS 更新程序 ===\n");
    println!("配置的域名数量: {}\n", config.domains.len());

    let mut client = CloudflareClient::new(&config);
    ensure_zone_id(&mut client, &config).await?;

    let local_ipv6 = get_local_ipv6_address(config.target_net_card.clone())?;

    println!("正在查询 Cloudflare DNS 记录...\n");
    let dns_records = client.get_dns_records().await?;

    process_all_domains(&config, &local_ipv6, &client, &dns_records).await;

    Ok(())
}
