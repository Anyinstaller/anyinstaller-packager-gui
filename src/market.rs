//! Marketplace catalog, region detection, GitHub acceleration, downloads, and safe installs.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use rune::Waker;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::settings::GithubAccelerationSettings;

pub const CATALOG_URL: &str =
    "https://cdn.jsdelivr.net/gh/Anyinstaller/client-backend-marketplace@main/catalog.toml";
pub const REGION_URL: &str = "https://ipwho.is/";
pub const DEFAULT_GITHUB_ACCELERATOR: &str = "https://gh.meali.top/";
const USER_AGENT: &str = concat!("Anyinstaller-Packager-GUI/", env!("CARGO_PKG_VERSION"));
const MAX_CATALOG_BYTES: u64 = 256 * 1024;
const MAX_REGION_BYTES: u64 = 64 * 1024;
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct Catalog {
    pub schema: u32,
    #[serde(default)]
    pub updated: String,
    pub marketplace: MarketplaceInfo,
    #[serde(default)]
    pub components: Vec<Component>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MarketplaceInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub contribute: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub kind: ComponentKind,
    pub version: String,
    pub repository: String,
    pub release_tag: String,
    pub asset: String,
    pub sha256: String,
    pub license: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub source_repository: String,
    pub executable: String,
    #[serde(default)]
    pub assets_dir: Option<String>,
    #[serde(default)]
    pub official: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    Client,
    Backend,
    Stub,
}

impl ComponentKind {
    pub fn label(self) -> &'static str {
        match self {
            ComponentKind::Client => "客户端",
            ComponentKind::Backend => "后端",
            ComponentKind::Stub => "STUB",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegionInfo {
    pub ip: String,
    pub country_code: String,
    pub country: String,
    pub domestic: bool,
    pub fallback: bool,
}

#[derive(Clone, Debug)]
pub struct InstalledComponent {
    pub component: Component,
    pub executable: PathBuf,
    pub assets_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum MarketEvent {
    CatalogLoaded(Catalog),
    CatalogFailed(String),
    RegionLoaded(RegionInfo),
    DownloadMessage {
        component_id: String,
        message: String,
    },
    DownloadProgress {
        component_id: String,
        received: u64,
        total: Option<u64>,
    },
    DownloadInstalled(Box<InstalledComponent>),
    DownloadFailed {
        component_id: String,
        message: String,
    },
}

#[derive(Deserialize)]
struct RegionResponse {
    success: bool,
    #[serde(default)]
    ip: String,
    #[serde(default)]
    country_code: String,
    #[serde(default)]
    country: String,
    #[serde(default)]
    message: Option<String>,
}

pub fn start_catalog_refresh(sender: Sender<MarketEvent>, wake: Waker) {
    std::thread::spawn(move || {
        let result = fetch_catalog();
        let event = match result {
            Ok(catalog) => MarketEvent::CatalogLoaded(catalog),
            Err(error) => MarketEvent::CatalogFailed(format!("{error:#}")),
        };
        send(&sender, &wake, event);
    });
}

pub fn start_region_check(sender: Sender<MarketEvent>, wake: Waker) {
    std::thread::spawn(move || {
        let event = MarketEvent::RegionLoaded(match fetch_region() {
            Ok(region) => region,
            Err(error) => fallback_region(&format!("{error:#}")),
        });
        send(&sender, &wake, event);
    });
}

pub fn start_download(
    component: Component,
    acceleration: GithubAccelerationSettings,
    sender: Sender<MarketEvent>,
    wake: Waker,
) {
    std::thread::spawn(move || {
        let id = component.id.clone();
        let result = download_and_install(&component, &acceleration, &sender, &wake);
        let event = match result {
            Ok(installed) => MarketEvent::DownloadInstalled(Box::new(installed)),
            Err(error) => MarketEvent::DownloadFailed {
                component_id: id,
                message: format!("{error:#}"),
            },
        };
        send(&sender, &wake, event);
    });
}

pub fn fetch_catalog() -> Result<Catalog> {
    let agent = http_agent(Duration::from_secs(30));
    let mut response = agent
        .get(CATALOG_URL)
        .header("Accept", "application/toml,text/plain")
        .call()
        .context("无法从 jsDelivr 获取市场目录")?;
    let text = response
        .body_mut()
        .with_config()
        .limit(MAX_CATALOG_BYTES)
        .read_to_string()
        .context("读取市场目录失败")?;
    let catalog: Catalog = toml::from_str(&text).context("catalog.toml 格式无效")?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

pub fn fetch_region() -> Result<RegionInfo> {
    let agent = http_agent(Duration::from_secs(12));
    let mut response = agent
        .get(REGION_URL)
        .header("Accept", "application/json")
        .call()
        .context("出口 IP 检测请求失败")?;
    let text = response
        .body_mut()
        .with_config()
        .limit(MAX_REGION_BYTES)
        .read_to_string()
        .context("读取 ipwho.is 响应失败")?;
    let response: RegionResponse = serde_json::from_str(&text).context("ipwho.is 响应格式无效")?;
    if !response.success {
        bail!(
            "ipwho.is 检测失败：{}",
            response.message.unwrap_or_else(|| "unknown error".into())
        );
    }
    if response.country_code.is_empty() {
        bail!("ipwho.is 响应缺少 country_code");
    }
    let country_code = response.country_code.to_ascii_uppercase();
    Ok(RegionInfo {
        ip: if response.ip.is_empty() {
            "未知".into()
        } else {
            response.ip
        },
        domestic: country_code == "CN",
        country_code,
        country: response.country,
        fallback: false,
    })
}

fn fallback_region(error: &str) -> RegionInfo {
    RegionInfo {
        ip: "不可用".into(),
        country_code: "CN".into(),
        country: format!("ipwho.is 请求失败：{error}"),
        domestic: true,
        fallback: true,
    }
}

pub fn normalize_accelerator_prefix(value: &str) -> Result<String> {
    let mut value = value.trim().to_string();
    let lower = value.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        bail!("GitHub 加速地址必须以 http:// 或 https:// 开头");
    }
    if value
        .chars()
        .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        bail!("GitHub 加速地址不能包含空白或控制字符");
    }
    if !value.ends_with('/') {
        value.push('/');
    }
    Ok(value)
}

pub fn direct_release_url(component: &Component) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/{}",
        component.repository, component.release_tag, component.asset
    )
}

pub fn download_urls(
    component: &Component,
    acceleration: &GithubAccelerationSettings,
) -> Result<Vec<String>> {
    let direct = direct_release_url(component);
    if !acceleration.enabled {
        return Ok(vec![direct]);
    }
    let prefix = normalize_accelerator_prefix(&acceleration.prefix)?;
    Ok(vec![format!("{prefix}{direct}"), direct])
}

pub fn installed_component(component: &Component) -> Option<InstalledComponent> {
    let root = final_install_dir(component)?;
    let executable = root.join(component.executable.replace('\\', "/"));
    if !executable.is_file() {
        return None;
    }
    let assets_dir = component
        .assets_dir
        .as_ref()
        .map(|assets| root.join(assets.replace('\\', "/")));
    if assets_dir.as_ref().is_some_and(|assets| !assets.is_dir()) {
        return None;
    }
    Some(InstalledComponent {
        component: component.clone(),
        executable,
        assets_dir,
    })
}

fn download_and_install(
    component: &Component,
    acceleration: &GithubAccelerationSettings,
    sender: &Sender<MarketEvent>,
    wake: &Waker,
) -> Result<InstalledComponent> {
    if let Some(existing) = installed_component(component) {
        send(
            sender,
            wake,
            MarketEvent::DownloadMessage {
                component_id: component.id.clone(),
                message: "已存在经过校验的缓存，直接复用。".into(),
            },
        );
        return Ok(existing);
    }

    let cache = cache_root().join(&component.id);
    std::fs::create_dir_all(&cache)
        .with_context(|| format!("无法创建市场缓存目录：{}", cache.display()))?;
    let stamp = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let zip_path = cache.join(format!(".download-{stamp}.zip"));
    let urls = download_urls(component, acceleration)?;
    let agent = http_agent(Duration::from_secs(300));
    let mut last_error = None;

    for url in urls {
        let accelerated = url != direct_release_url(component);
        send(
            sender,
            wake,
            MarketEvent::DownloadMessage {
                component_id: component.id.clone(),
                message: if accelerated {
                    "正在通过 GitHub 加速下载...".into()
                } else {
                    "正在从 GitHub 直连下载...".into()
                },
            },
        );
        match download_to_file(&agent, &url, &zip_path, component, sender, wake) {
            Ok(()) => match verify_sha256(&zip_path, &component.sha256) {
                Ok(()) => {
                    last_error = None;
                    break;
                }
                Err(error) => {
                    let _ = std::fs::remove_file(&zip_path);
                    last_error = Some(error);
                    if accelerated {
                        send(
                            sender,
                            wake,
                            MarketEvent::DownloadMessage {
                                component_id: component.id.clone(),
                                message: "加速下载的 SHA-256 不匹配，回退到 GitHub 直连。".into(),
                            },
                        );
                    }
                }
            },
            Err(error) => {
                last_error = Some(error);
                if accelerated {
                    send(
                        sender,
                        wake,
                        MarketEvent::DownloadMessage {
                            component_id: component.id.clone(),
                            message: "加速下载失败，回退到 GitHub 直连。".into(),
                        },
                    );
                }
            }
        }
    }

    if let Some(error) = last_error {
        let _ = std::fs::remove_file(&zip_path);
        return Err(error);
    }

    let final_dir = final_install_dir(component).context("组件缓存路径无效")?;
    if !final_dir.exists() {
        let extract_dir = cache.join(format!(".extract-{stamp}"));
        extract_zip(&zip_path, &extract_dir)?;
        let executable = extract_dir.join(component.executable.replace('\\', "/"));
        if !executable.is_file() {
            bail!("压缩包中缺少可执行文件：{}", component.executable);
        }
        if let Some(assets) = &component.assets_dir {
            let assets = extract_dir.join(assets.replace('\\', "/"));
            if !assets.is_dir() {
                bail!("压缩包中缺少资源目录：{}", assets.display());
            }
        }
        if final_dir.exists() {
            bail!("组件缓存被其他进程同时创建：{}", final_dir.display());
        }
        std::fs::rename(&extract_dir, &final_dir)
            .with_context(|| format!("无法提交组件缓存：{}", final_dir.display()))?;
    }
    let _ = std::fs::remove_file(&zip_path);
    installed_component(component).context("组件已下载但没有找到可执行文件")
}

fn download_to_file(
    agent: &ureq::Agent,
    url: &str,
    path: &Path,
    component: &Component,
    sender: &Sender<MarketEvent>,
    wake: &Waker,
) -> Result<()> {
    let response = agent
        .get(url)
        .header("Accept", "application/zip,application/octet-stream")
        .call()
        .with_context(|| format!("下载失败：{url}"))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if total.is_some_and(|total| total > MAX_DOWNLOAD_BYTES) {
        bail!("下载文件超过 512 MiB 限制");
    }

    let mut reader = response
        .into_body()
        .into_with_config()
        .limit(MAX_DOWNLOAD_BYTES + 1)
        .reader();
    let file =
        File::create(path).with_context(|| format!("无法创建下载临时文件：{}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let mut buffer = [0u8; 64 * 1024];
    let mut received = 0u64;
    let mut last_emit = Instant::now();
    loop {
        let count = reader.read(&mut buffer).context("读取下载内容失败")?;
        if count == 0 {
            break;
        }
        received += count as u64;
        if received > MAX_DOWNLOAD_BYTES {
            bail!("下载文件超过 512 MiB 限制");
        }
        writer
            .write_all(&buffer[..count])
            .context("写入下载文件失败")?;
        if last_emit.elapsed() >= Duration::from_millis(120) {
            send(
                sender,
                wake,
                MarketEvent::DownloadProgress {
                    component_id: component.id.clone(),
                    received,
                    total,
                },
            );
            last_emit = Instant::now();
        }
    }
    writer.flush().context("刷新下载文件失败")?;
    send(
        sender,
        wake,
        MarketEvent::DownloadProgress {
            component_id: component.id.clone(),
            received,
            total,
        },
    );
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut file = File::open(path).context("无法打开下载文件进行哈希校验")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).context("读取下载文件失败")?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected) {
        bail!("SHA-256 校验失败：期望 {expected}，实际 {actual}");
    }
    Ok(())
}

fn extract_zip(zip_path: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)
        .with_context(|| format!("无法创建解压目录：{}", destination.display()))?;
    let file = File::open(zip_path).context("无法打开组件 ZIP")?;
    let mut archive = ZipArchive::new(file).context("组件 ZIP 格式无效")?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        bail!("组件 ZIP 条目过多");
    }
    let mut total_size = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).context("读取 ZIP 条目失败")?;
        let entry_name = entry
            .name()
            .map(|name| name.into_owned())
            .unwrap_or_else(|_| "<invalid-name>".into());
        if entry.encrypted() {
            bail!("组件 ZIP 不能包含加密条目");
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("组件 ZIP 不能包含符号链接");
        }
        let enclosed = entry
            .enclosed_name()
            .with_context(|| format!("ZIP 条目路径不安全：{entry_name}"))?;
        if enclosed
            .components()
            .any(|component| !matches!(component, PathComponent::Normal(_)))
        {
            bail!("ZIP 条目路径包含非普通组件：{}", enclosed.display());
        }
        let target = destination.join(&enclosed);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("无法创建目录：{}", target.display()))?;
            continue;
        }
        if entry.size() > MAX_ARCHIVE_ENTRY_BYTES {
            bail!("组件 ZIP 单文件超过 512 MiB");
        }
        total_size = total_size.saturating_add(entry.size());
        if total_size > MAX_ARCHIVE_BYTES {
            bail!("组件 ZIP 解压后超过 1 GiB");
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建目录：{}", parent.display()))?;
        }
        let mut output =
            File::create(&target).with_context(|| format!("无法创建文件：{}", target.display()))?;
        let copied = std::io::copy(&mut entry, &mut output)
            .with_context(|| format!("解压文件失败：{}", target.display()))?;
        if copied != entry.size() {
            bail!("ZIP 条目大小与声明不一致：{entry_name}");
        }
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<()> {
    if catalog.schema != 1 {
        bail!("不支持的市场目录 schema：{}", catalog.schema);
    }
    if catalog.components.is_empty() {
        bail!("市场目录没有组件");
    }
    validate_http_url(&catalog.marketplace.homepage, "marketplace.homepage")?;
    validate_http_url(&catalog.marketplace.contribute, "marketplace.contribute")?;
    let mut ids = HashSet::new();
    for component in &catalog.components {
        validate_component(component)?;
        if !ids.insert(component.id.to_ascii_lowercase()) {
            bail!("市场目录包含重复组件 id：{}", component.id);
        }
    }
    Ok(())
}

fn validate_component(component: &Component) -> Result<()> {
    validate_id(&component.id, "component.id")?;
    if component.name.trim().is_empty() || component.version.trim().is_empty() {
        bail!("组件 {} 缺少 name 或 version", component.id);
    }
    validate_release_part(&component.version, "version")?;
    validate_repository(&component.repository)?;
    if !component.source_repository.is_empty() {
        validate_repository(&component.source_repository)?;
    }
    validate_release_part(&component.release_tag, "release_tag")?;
    validate_release_part(&component.asset, "asset")?;
    if component.sha256.len() != 64
        || !component
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("组件 {} 的 sha256 无效", component.id);
    }
    if component.license.trim().is_empty()
        || component.license.len() > 80
        || component.license.chars().any(char::is_control)
    {
        bail!("组件 {} 的 license 无效", component.id);
    }
    validate_relative_file(&component.executable, "executable")?;
    if let Some(assets) = &component.assets_dir {
        validate_relative_file(assets, "assets_dir")?;
    }
    if component.kind == ComponentKind::Client && component.assets_dir.is_none() {
        bail!("client 组件 {} 必须提供 assets_dir", component.id);
    }
    if !component.homepage.is_empty() {
        validate_http_url(&component.homepage, "component.homepage")?;
    }
    Ok(())
}

fn validate_id(value: &str, field: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 96
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        bail!("{field} 只能包含 ASCII 字母、数字、点、下划线和连字符");
    }
    Ok(())
}

fn validate_repository(value: &str) -> Result<()> {
    let mut parts = value.split('/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();
    if owner.is_empty()
        || repo.is_empty()
        || parts.next().is_some()
        || !owner
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        || !repo
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        bail!("GitHub 仓库名称必须使用 owner/repo 格式：{value}");
    }
    Ok(())
}

fn validate_release_part(value: &str, field: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 255
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        bail!("{field} 包含不支持的字符：{value}");
    }
    Ok(())
}

fn validate_relative_file(value: &str, field: &str) -> Result<()> {
    let path = Path::new(value);
    if path.is_absolute()
        || value.is_empty()
        || path
            .components()
            .any(|component| !matches!(component, PathComponent::Normal(_)))
    {
        bail!("{field} 必须是安全的相对路径");
    }
    Ok(())
}

fn validate_http_url(value: &str, field: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://"))
        || value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        bail!("{field} 必须是有效的 http(s) URL");
    }
    Ok(())
}

fn http_agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

fn final_install_dir(component: &Component) -> Option<PathBuf> {
    if component.id.is_empty()
        || component.version.is_empty()
        || component.sha256.len() < 8
        || !component
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return None;
    }
    Some(cache_root().join(&component.id).join(format!(
        "{}-{}",
        component.version,
        component.sha256[..8].to_ascii_lowercase()
    )))
}

pub fn cache_root() -> PathBuf {
    crate::settings::data_root().join("marketplace")
}

fn send(sender: &Sender<MarketEvent>, wake: &Waker, event: MarketEvent) {
    if sender.send(event).is_ok() {
        wake();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component() -> Component {
        Component {
            id: "official-test".into(),
            name: "Test".into(),
            kind: ComponentKind::Backend,
            version: "1.0.0".into(),
            repository: "Anyinstaller/client-backend-marketplace".into(),
            release_tag: "components-v1.0.0".into(),
            asset: "backend-windows-x64.zip".into(),
            sha256: "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF".into(),
            license: "MPL-2.0".into(),
            homepage: "https://github.com/Anyinstaller/voidbackend".into(),
            source_repository: "Anyinstaller/voidbackend".into(),
            executable: "backend.exe".into(),
            assets_dir: None,
            official: true,
            description: "Test backend".into(),
        }
    }

    #[test]
    fn release_urls_use_acceleration_then_direct_fallback() {
        let component = component();
        let urls = download_urls(
            &component,
            &GithubAccelerationSettings {
                enabled: true,
                prefix: "https://gh.meali.top".into(),
            },
        )
        .unwrap();
        assert_eq!(urls.len(), 2);
        assert_eq!(
            urls[0],
            "https://gh.meali.top/https://github.com/Anyinstaller/client-backend-marketplace/releases/download/components-v1.0.0/backend-windows-x64.zip"
        );
        assert!(urls[1].starts_with("https://github.com/"));
    }

    #[test]
    fn disabled_acceleration_uses_direct_url_only() {
        let urls = download_urls(&component(), &GithubAccelerationSettings::default()).unwrap();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].starts_with("https://github.com/"));
    }

    #[test]
    #[ignore = "queries ipwho.is over the network"]
    fn detects_exit_region_from_ipwho() {
        let region = fetch_region().unwrap();
        assert!(!region.country_code.is_empty());
        assert!(!region.ip.is_empty());
        assert_eq!(region.domestic, region.country_code == "CN");
    }

    #[test]
    #[ignore = "downloads the official backend release over the network"]
    fn downloads_and_installs_official_backend_from_release() {
        let catalog = fetch_catalog().unwrap();
        let backend = catalog
            .components
            .iter()
            .find(|component| component.id == "official-voidbackend")
            .unwrap();
        let (sender, _receiver) = std::sync::mpsc::channel();
        let wake: Waker = std::sync::Arc::new(|| {});
        let installed = download_and_install(
            backend,
            &GithubAccelerationSettings::default(),
            &sender,
            &wake,
        )
        .unwrap();
        assert!(installed.executable.is_file());
        assert!(installed.executable.ends_with("backend.exe"));
    }

    #[test]
    fn checked_in_catalog_and_release_assets_validate_in_this_workspace() {
        let catalog_path = Path::new("../client-backend-marketplace/catalog.toml");
        if !catalog_path.is_file() {
            return;
        }
        let text = std::fs::read_to_string(catalog_path).unwrap();
        let catalog: Catalog = toml::from_str(&text).unwrap();
        validate_catalog(&catalog).unwrap();
        for component in &catalog.components {
            let asset = Path::new("../client-backend-marketplace/dist").join(&component.asset);
            if asset.is_file() {
                verify_sha256(&asset, &component.sha256).unwrap();
            }
        }
    }

    #[test]
    fn rejects_path_traversal_in_component_version() {
        let mut component = component();
        component.version = "../escape".into();
        assert!(validate_component(&component).is_err());
    }

    #[test]
    fn ipwho_failure_defaults_to_domestic() {
        let region = fallback_region("offline");
        assert!(region.domestic);
        assert!(region.fallback);
        assert_eq!(region.country_code, "CN");
    }

    #[test]
    fn validates_component_fields() {
        assert!(validate_component(&component()).is_ok());
    }
}
