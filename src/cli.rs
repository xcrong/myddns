use clap::Parser;
use std::error::Error;

use crate::utils::list_all_netcards;

#[derive(Parser)]
#[command(name = "myddns")]
#[command(about = "Cloudflare IPv6 DDNS 更新程序", long_about = None)]
pub struct Cli {
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,

    #[arg(short = 't', long, help = "测试配置文件是否能正确解析")]
    pub test: bool,

    #[arg(long, value_name = "FILE", help = "测试指定的配置文件路径")]
    pub test_config_path: Option<String>,

    #[arg(short = 'n', long, help = "列出所有可用网卡及其 IPv6 地址")]
    pub netcards: bool,
}

pub fn handle_netcards_command() -> Result<(), Box<dyn Error>> {
    println!("=== 可用网卡列表 ===\n");
    let netcards = list_all_netcards()?;

    if netcards.is_empty() {
        println!("未找到任何带有 IPv6 地址的网卡");
        return Ok(());
    }

    for (name, addresses) in netcards {
        println!("{}:", name);
        for addr in addresses {
            println!("  - {}", addr);
        }
        println!();
    }

    Ok(())
}

pub fn handle_test_command(test_config_path: Option<String>) -> Result<(), Box<dyn Error>> {
    let config_fpath = test_config_path.unwrap_or_else(|| "/etc/myddns/config.toml".to_string());
    crate::config::test_config(&config_fpath)
}
