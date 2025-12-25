use crate::model::Config;
use std::error::Error;
use std::fs;

pub fn load_config(config_fpath: &str) -> Result<Config, Box<dyn Error>> {
    let toml_content = fs::read_to_string(config_fpath)?;
    let config: Config = toml::from_str(&toml_content)?;
    Ok(config)
}

pub fn test_config(config_fpath: &str) -> Result<(), Box<dyn Error>> {
    println!("=== 配置文件测试 ===\n");
    println!("配置文件路径: {}\n", config_fpath);

    let config = load_config(config_fpath)?;

    println!("✓ 配置文件解析成功\n");
    println!("配置摘要:");
    println!("  目标网卡: {}", config.target_net_card);
    println!(
        "  Zone ID: {}",
        config.zone_id.as_deref().unwrap_or("未配置")
    );
    println!(
        "  API Token: {}...",
        &config.api_token[..config.api_token.len().min(20)]
    );
    println!("  Base URL: {}", config.base_url);
    println!("  域名数量: {}\n", config.domains.len());

    for (index, domain) in config.domains.iter().enumerate() {
        println!("  [{}] {}", index + 1, domain.domain);
        println!(
            "      代理: {}",
            if domain.proxied {
                "启用"
            } else {
                "未启用"
            }
        );
    }

    println!("\n✓ 配置文件验证通过");
    Ok(())
}
