# myddns

一个用 Rust 编写的 Cloudflare IPv6 DDNS 更新程序，可以自动将本地 IPv6 地址同步到 Cloudflare DNS。

## 功能特性

- 自动获取指定网卡的 IPv6 地址
- 批量更新多个域名的 AAAA 记录
- 自动或手动配置 Cloudflare Zone ID
- Zone ID 缓存机制（24小时有效期）
- 支持 Cloudflare 代理（Proxied）配置
- 自动创建不存在的 DNS 记录
- 智能检测 IP 和代理状态变化，仅在必要时更新
- 提供网卡列表查看功能
- 配置文件测试功能

## 安装

### 从源码编译

```bash
# 克隆或下载项目
cd myddns

# 编译发布版本
cargo build --release

# 编译后的二进制文件位于 target/release/myddns
```

## 配置

### 1. 创建配置文件

将示例配置文件复制到目标位置：

```bash
sudo mkdir -p /etc/myddns
sudo cp config.toml.example /etc/myddns/config.toml
```

### 2. 编辑配置文件

```bash
sudo nano /etc/myddns/config.toml
```

配置文件示例：

```toml
# 目标网卡名称
target_net_card = "wlp2s0"

# Zone ID 配置（可选）
# 如果留空，程序会自动根据域名查找对应的 Zone ID
# 如果手动配置，则直接使用该 Zone ID
zone_id = ""

# Cloudflare API Token
# 需要有 Zone:Read 和 DNS:Edit 权限
api_token = "your_api_token_here"

# Cloudflare API 基础 URL（通常不需要修改）
base_url = "https://api.cloudflare.com/client/v4"

# 域名配置列表
[[domains]]
domain = "a.example.me"
proxied = false

[[domains]]
domain = "b.example.me"
proxied = true
```

### 3. 获取 Cloudflare API Token

1. 登录 [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. 进入 "My Profile" → "API Tokens"
3. 点击 "Create Token"
4. 使用 "Edit zone DNS" 模板或自定义创建
5. 确保权限包含：
   - **Zone** - `Zone` - `Read`
   - **DNS** - `DNS` - `Edit`
6. 选择需要管理的 Zone（或选择 All zones）
7. 创建并复制 Token 到配置文件

### 4. 获取 Zone ID（可选）

如果不想让程序自动获取 Zone ID，可以手动配置：

```bash
curl https://api.cloudflare.com/client/v4/zones \
    -H "Authorization: Bearer YOUR_API_TOKEN"
```

返回结果中的 `id` 字段即为 Zone ID。

## 使用方法

### 查看所有可用网卡及其 IPv6 地址

```bash
./target/release/myddns --netcards
```

输出示例：
```
=== 可用网卡列表 ===

eth0:
  - 2001:db8::1
  - fe80::1

wlp2s0:
  - 2001:db8::2
  - fe80::2
```

### 测试配置文件

```bash
# 测试默认配置文件 /etc/myddns/config.toml
./target/release/myddns --test

# 测试指定配置文件
./target/release/myddns --test-config-path /path/to/config.toml
```

### 更新 DNS 记录

```bash
# 使用默认配置文件
./target/release/myddns

# 使用指定配置文件
./target/release/myddns --config /path/to/config.toml
```

### 命令行参数

```
Cloudflare IPv6 DDNS 更新程序

Usage: myddns [OPTIONS]

Options:
  -c, --config <FILE>      指定配置文件路径 [默认: /etc/myddns/config.toml]
  -t, --test               测试配置文件是否能正确解析
      --test-config-path <FILE>  测试指定的配置文件路径
  -n, --netcards           列出所有可用网卡及其 IPv6 地址
  -h, --help               显示帮助信息
  -V, --version            显示版本信息
```

## 工作原理

1. **获取本地 IPv6 地址**：从指定的网卡读取 IPv6 地址，优先使用全局单播地址（非 fe80:: 开头）
2. **获取 Zone ID**：如果配置文件中未指定，则通过 Cloudflare API 自动查找
3. **查询 DNS 记录**：从 Cloudflare 获取所有 DNS 记录
4. **对比和更新**：
   - 如果 AAAA 记录不存在，则创建新记录
   - 如果 IP 地址或代理状态发生变化，则更新记录
   - 如果记录已是最新，则跳过更新

## 定时任务

可以使用 cron 或 systemd timer 定期运行此程序：

### 使用 Cron

```bash
# 编辑 crontab
crontab -e

# 添加以下行，每 5 分钟运行一次
*/5 * * * * /path/to/myddns
```

### 使用 Systemd Timer

创建服务文件 `/etc/systemd/system/myddns.service`：

```ini
[Unit]
Description=Cloudflare IPv6 DDNS Update Service
After=network-online.target

[Service]
Type=oneshot
ExecStart=/path/to/myddns
```

创建定时器文件 `/etc/systemd/system/myddns.timer`：

```ini
[Unit]
Description=Run myddns every 5 minutes
Requires=myddns.service

[Timer]
OnCalendar=*:0/5
Persistent=true

[Install]
WantedBy=timers.target
```

启用并启动定时器：

```bash
sudo systemctl enable myddns.timer
sudo systemctl start myddns.timer
```

## 故障排除

### 未找到 IPv6 地址

- 使用 `--netcards` 参数查看所有网卡及其 IPv6 地址
- 确认配置文件中的 `target_net_card` 正确
- 确认网卡已启用且有 IPv6 地址

### API 请求失败

- 检查 API Token 是否正确
- 确认 API Token 有足够的权限（Zone:Read 和 DNS:Edit）
- 检查网络连接是否正常

### 未找到域名对应的 Zone

- 确认域名已在 Cloudflare 中添加
- 手动配置 `zone_id` 而非自动获取
- 检查 API Token 是否有读取 Zones 的权限

