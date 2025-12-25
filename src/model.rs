use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn default_base_url() -> String {
    "https://api.cloudflare.com/client/v4".to_string()
}

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    pub target_net_card: String,
    pub zone_id: Option<String>,
    pub api_token: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    pub domains: Vec<DomainConfig>,
}

pub struct CloudflareClient {
    api_token: String,
    base_url: String,
    zone_id: Option<String>,
    client: reqwest::Client,
}

impl CloudflareClient {
    pub fn new(config: &Config) -> Self {
        Self {
            api_token: config.api_token.clone(),
            base_url: config.base_url.clone(),
            zone_id: config.zone_id.clone(),
            client: reqwest::Client::new(),
        }
    }

    pub fn set_zone_id(&mut self, zone_id: String) {
        self.zone_id = Some(zone_id);
    }

    pub fn zone_id(&self) -> &Option<String> {
        &self.zone_id
    }

    pub async fn get_dns_records(&self) -> Result<Vec<DnsRecord>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/zones/{}/dns_records",
            self.base_url,
            self.zone_id.as_ref().ok_or("Zone ID not set")?
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            return Err(format!("API 请求失败 ({}): {}", status, text).into());
        }

        let cf_response: CloudflareResponse = resp.json().await?;

        if !cf_response.success {
            return Err(format!("Cloudflare API 返回错误: {:?}", cf_response.errors).into());
        }

        Ok(cf_response.result)
    }

    pub async fn update_dns_record(
        &self,
        record_id: &str,
        new_ipv6: &str,
        domain: &DomainConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "{}/zones/{}/dns_records/{}",
            self.base_url,
            self.zone_id.as_ref().ok_or("Zone ID not set")?,
            record_id
        );

        let update_data = UpdateDnsRecord {
            record_type: "AAAA".to_string(),
            name: domain.domain.to_string(),
            content: new_ipv6.to_string(),
            ttl: 120,
            proxied: domain.proxied,
        };

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&update_data)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            return Err(format!("更新 DNS 记录失败 ({}): {}", status, text).into());
        }

        Ok(())
    }

    pub async fn create_dns_record(
        &self,
        ipv6: &str,
        domain: &DomainConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "{}/zones/{}/dns_records",
            self.base_url,
            self.zone_id.as_ref().ok_or("Zone ID not set")?
        );

        let create_data = UpdateDnsRecord {
            record_type: "AAAA".to_string(),
            name: domain.domain.to_string(),
            content: ipv6.to_string(),
            ttl: 120,
            proxied: domain.proxied,
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&create_data)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            return Err(format!("创建 DNS 记录失败 ({}): {}", status, text).into());
        }

        Ok(())
    }

    pub async fn get_zone_id_by_domain(
        &self,
        domain: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(cached_zone_id) = read_cache(domain) {
            println!("  ✓ 从缓存读取 Zone ID: {}", cached_zone_id);
            return Ok(cached_zone_id);
        }

        let url = format!("{}/zones", self.base_url);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            return Err(format!("获取 Zone 列表失败 ({}): {}", status, text).into());
        }

        let cf_response: ZoneListResponse = resp.json().await?;

        if !cf_response.success {
            return Err(format!("Cloudflare API 返回错误: {:?}", cf_response.errors).into());
        }

        if cf_response.result.is_empty() {
            return Err("未找到任何 Zone".into());
        }

        let root_domain = extract_root_domain(domain);

        let matched_zone = cf_response
            .result
            .iter()
            .find(|zone| zone.name == root_domain);

        match matched_zone {
            Some(zone) => {
                println!("  ✓ 找到匹配的 Zone: {} (ID: {})", zone.name, zone.id);
                write_cache(&zone.id, domain);
                Ok(zone.id.clone())
            }
            None => {
                let available_zones: Vec<&str> =
                    cf_response.result.iter().map(|z| z.name.as_str()).collect();
                Err(format!(
                    "未找到域名 {} 对应的 Zone。可用的 Zones: {:?}",
                    root_domain, available_zones
                )
                .into())
            }
        }
    }
}

fn extract_root_domain(domain: &str) -> String {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2..].join(".")
    } else {
        domain.to_string()
    }
}

// 域名配置结构体
#[derive(serde::Deserialize, Debug, Clone)]
pub struct DomainConfig {
    pub domain: String,
    pub proxied: bool,
}

#[derive(Debug, Deserialize)]
pub struct CloudflareResponse {
    pub success: bool,
    pub result: Vec<DnsRecord>,
    pub errors: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ZoneListResponse {
    pub success: bool,
    pub result: Vec<Zone>,
    pub errors: Vec<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct DnsRecord {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub content: String,
    pub proxied: bool,
    pub ttl: u32,
}

#[derive(Debug, Serialize)]
pub struct UpdateDnsRecord {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct ZoneIdCache {
    zone_id: String,
    domain: String,
    cached_at: u64,
}

fn get_cache_path() -> String {
    "/tmp/myddns_zone_id_cache.toml".to_string()
}

fn read_cache(domain: &str) -> Option<String> {
    let cache_path = get_cache_path();
    let cache_content = fs::read_to_string(cache_path).ok()?;
    let cache: ZoneIdCache = toml::from_str(&cache_content).ok()?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

    let cache_age = now - cache.cached_at;
    const CACHE_TTL: u64 = 24 * 60 * 60;

    if cache_age > CACHE_TTL {
        return None;
    }

    if cache.domain == domain {
        Some(cache.zone_id)
    } else {
        None
    }
}

fn write_cache(zone_id: &str, domain: &str) {
    let cache_path = get_cache_path();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let cache = ZoneIdCache {
        zone_id: zone_id.to_string(),
        domain: domain.to_string(),
        cached_at: now,
    };

    let toml_content = toml::to_string(&cache).unwrap();
    let _ = fs::write(cache_path, toml_content);
}
