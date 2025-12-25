use crate::model::{CloudflareClient, Config, DnsRecord, DomainConfig};
use get_if_addrs::get_if_addrs;
use std::error::Error;

pub async fn ensure_zone_id(
    client: &mut CloudflareClient,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    if client.zone_id().is_some() {
        println!(
            "使用配置的 Zone ID: {}\n",
            client.zone_id().as_ref().unwrap()
        );
        return Ok(());
    }

    println!("Zone ID 未配置，正在自动获取...\n");
    if config.domains.is_empty() {
        return Err("配置文件中没有域名，无法自动获取 Zone ID".into());
    }

    let first_domain = &config.domains[0].domain;
    println!("使用域名 {} 查找对应的 Zone...", first_domain);

    match client.get_zone_id_by_domain(first_domain).await {
        Ok(zone_id) => {
            client.set_zone_id(zone_id);
            println!("\n");
            Ok(())
        }
        Err(e) => {
            eprintln!("✗ 自动获取 Zone ID 失败: {}", e);
            eprintln!("  请在配置文件中手动设置 zone_id，或确保 API Token 有读取 Zones 的权限");
            Err(e)
        }
    }
}

pub fn get_local_ipv6_address(net_card: String) -> Result<String, Box<dyn Error>> {
    let addresses = get_ipv6_address(net_card)?;

    if addresses.is_empty() {
        eprintln!("✗ 未找到 IPv6 地址");
        return Err("未找到 IPv6 地址".into());
    }

    println!("✓ 找到本地 IPv6 地址:");
    for addr in &addresses {
        println!("  - {}", addr);
    }

    let global_addr = addresses
        .iter()
        .find(|addr| !addr.starts_with("fe80:"))
        .or_else(|| addresses.first())
        .unwrap();
    println!("\n使用地址: {}\n", global_addr);
    Ok(global_addr.clone())
}

pub async fn process_all_domains(
    config: &Config,
    local_ipv6: &str,
    client: &CloudflareClient,
    dns_records: &[DnsRecord],
) {
    for (index, domain) in config.domains.iter().enumerate() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "[{}/{}] 处理域名: {}",
            index + 1,
            config.domains.len(),
            domain.domain
        );
        println!(
            "  代理状态: {}",
            if domain.proxied {
                "已启用"
            } else {
                "未启用"
            }
        );

        match process_domain(domain, local_ipv6, client, dns_records).await {
            Ok(msg) => println!("  ✓ {}", msg),
            Err(e) => eprintln!("  ✗ 失败: {}", e),
        }
        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ 所有域名处理完成");
}

pub async fn process_domain(
    domain: &DomainConfig,
    local_ipv6: &str,
    client: &CloudflareClient,
    dns_records: &[DnsRecord],
) -> Result<String, Box<dyn Error>> {
    let target_record = dns_records
        .iter()
        .find(|record| record.name == domain.domain && record.record_type == "AAAA");

    match target_record {
        Some(record) => {
            println!("  当前 IP: {}", record.content);
            println!("  本地 IP: {}", local_ipv6);

            let ip_changed = record.content != local_ipv6;
            let proxy_changed = record.proxied != domain.proxied;

            if !ip_changed && !proxy_changed {
                return Ok("DNS 记录已是最新，无需更新".to_string());
            }

            if ip_changed {
                println!("  ⚠ IP 地址已变化");
            }
            if proxy_changed {
                println!(
                    "  ⚠ 代理状态需更新: {} -> {}",
                    record.proxied, domain.proxied
                );
            }

            client
                .update_dns_record(&record.id, local_ipv6, domain)
                .await?;
            Ok("DNS 记录更新成功".to_string())
        }
        None => {
            println!("  ⚠ 未找到 AAAA 记录，正在创建...");
            client.create_dns_record(local_ipv6, domain).await?;
            Ok("DNS 记录创建成功".to_string())
        }
    }
}

pub fn get_ipv6_address(net_card: String) -> Result<Vec<String>, Box<dyn Error>> {
    let if_addrs = get_if_addrs()?;
    let mut ipv6_addresses = Vec::new();

    for interface in if_addrs {
        if interface.name != net_card.to_string() {
            continue;
        }

        if interface.ip().is_ipv6() {
            ipv6_addresses.push(interface.ip().to_string());
        }
    }

    if ipv6_addresses.is_empty() {
        return Err(format!("网卡 {} 上未找到 IPv6 地址", net_card.to_string()).into());
    }

    Ok(ipv6_addresses)
}

pub fn list_all_netcards() -> Result<Vec<(String, Vec<String>)>, Box<dyn Error>> {
    let if_addrs = get_if_addrs()?;
    let mut netcards: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for interface in if_addrs {
        if interface.ip().is_ipv6() {
            let name = interface.name.clone();
            netcards
                .entry(name)
                .or_insert_with(Vec::new)
                .push(interface.ip().to_string());
        }
    }

    let mut result: Vec<(String, Vec<String>)> = netcards.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(result)
}
