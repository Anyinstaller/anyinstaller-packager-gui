//! Reading, validating, and writing AIP v1 `config.toml` files.
//!
//! The GUI deliberately edits only the keys defined by AIP v1. When a file is
//! loaded, its original document is retained as the render base so comments and
//! unknown forward-compatible keys survive a save.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use toml::Value;
use toml_edit::{value, DocumentMut, Item, Table};

pub const MAX_CONFIG_BYTES: u64 = 64 * 1024;
pub const MAX_NAME_CHARS: usize = 128;
const MAX_INSTALL_PATH_UTF16: usize = 32_767;

const DEFAULT_CONFIG: &str = r#"# Anyinstaller Protocol (AIP) v1 source configuration.
protocol = 1
name = "Example App"

[defaults]
installLocation = '%LocalAppData%\Programs\Example App'
createStartMenuFolder = true
createDesktopShortcut = true
"#;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigDraft {
    pub name: String,
    pub icon: String,
    pub background: String,
    pub install_location: String,
    pub create_start_menu_folder: bool,
    pub create_desktop_shortcut: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigField {
    Name,
    Icon,
    Background,
    InstallLocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigProblem {
    pub field: ConfigField,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub draft: ConfigDraft,
    pub source: String,
}

pub fn load(path: &Path) -> Result<LoadedConfig> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    let source = decode(&bytes)?;
    let draft = parse(&source)?;
    Ok(LoadedConfig { draft, source })
}

pub fn parse(source: &str) -> Result<ConfigDraft> {
    if source.len() as u64 > MAX_CONFIG_BYTES {
        bail!("config.toml is larger than 64 KiB");
    }
    let value: Value = toml::from_str(source).context("config.toml is not valid TOML")?;
    let table = value
        .as_table()
        .context("config.toml root must be a TOML table")?;

    let protocol = match table.get("protocol") {
        Some(Value::Integer(value)) => *value,
        Some(_) => bail!("protocol must be an integer"),
        None => bail!("protocol is required"),
    };
    if protocol != 1 {
        bail!("unsupported AIP protocol number: {protocol}");
    }

    let name = match table.get("name") {
        Some(Value::String(value)) => value.clone(),
        Some(_) => bail!("name must be a string"),
        None => bail!("name is required"),
    };

    let icon = optional_string(table.get("icon"), "icon")?;
    let background = optional_string(table.get("background"), "background")?;

    let defaults = match table.get("defaults") {
        Some(Value::Table(value)) => value,
        Some(_) => bail!("defaults must be a table"),
        None => bail!("[defaults] is required"),
    };
    let install_location = match defaults.get("installLocation") {
        Some(Value::String(value)) => value.clone(),
        Some(_) => bail!("defaults.installLocation must be a string"),
        None => bail!("defaults.installLocation is required"),
    };
    let create_start_menu_folder = match defaults.get("createStartMenuFolder") {
        Some(Value::Boolean(value)) => *value,
        Some(_) => bail!("defaults.createStartMenuFolder must be a boolean"),
        None => bail!("defaults.createStartMenuFolder is required"),
    };
    let create_desktop_shortcut = match defaults.get("createDesktopShortcut") {
        Some(Value::Boolean(value)) => *value,
        Some(_) => bail!("defaults.createDesktopShortcut must be a boolean"),
        None => bail!("defaults.createDesktopShortcut is required"),
    };

    let draft = ConfigDraft {
        name,
        icon,
        background,
        install_location,
        create_start_menu_folder,
        create_desktop_shortcut,
    };
    if let Some(problem) = validate(&draft).into_iter().next() {
        bail!(problem.message);
    }
    Ok(draft)
}

pub fn render(draft: &ConfigDraft, base: Option<&str>) -> Result<String> {
    let mut document = base
        .unwrap_or(DEFAULT_CONFIG)
        .parse::<DocumentMut>()
        .context("base config.toml is not valid TOML")?;

    document["protocol"] = value(1_i64);
    document["name"] = value(draft.name.trim());

    set_optional(&mut document, "icon", draft.icon.trim());
    set_optional(&mut document, "background", draft.background.trim());

    let defaults_item = document
        .entry("defaults")
        .or_insert(Item::Table(Table::new()));
    let defaults = defaults_item
        .as_table_mut()
        .context("[defaults] must be a table")?;
    defaults["installLocation"] = value(draft.install_location.trim());
    defaults["createStartMenuFolder"] = value(draft.create_start_menu_folder);
    defaults["createDesktopShortcut"] = value(draft.create_desktop_shortcut);

    let rendered = document.to_string();
    if rendered.len() as u64 > MAX_CONFIG_BYTES {
        bail!("rendered config.toml is larger than 64 KiB");
    }
    Ok(rendered)
}

pub fn validate(draft: &ConfigDraft) -> Vec<ConfigProblem> {
    let mut problems = Vec::new();

    let name = draft.name.trim();
    if name.is_empty() {
        problems.push(problem(ConfigField::Name, "应用名称不能为空"));
    } else if name.chars().count() > MAX_NAME_CHARS {
        problems.push(problem(
            ConfigField::Name,
            format!("应用名称不能超过 {MAX_NAME_CHARS} 个字符"),
        ));
    } else if name.chars().any(char::is_control) {
        problems.push(problem(ConfigField::Name, "应用名称不能包含控制字符"));
    }

    if let Err(error) = validate_install_location(draft.install_location.trim()) {
        problems.push(problem(ConfigField::InstallLocation, error.to_string()));
    }

    let icon = draft.icon.trim();
    if !icon.is_empty() {
        if let Err(error) = validate_source_image_path(icon) {
            problems.push(problem(ConfigField::Icon, error.to_string()));
        }
    }

    let background = draft.background.trim();
    if !background.is_empty() && !is_special_background(background) && !is_hex_color(background) {
        if let Err(error) = validate_source_image_path(background) {
            problems.push(problem(ConfigField::Background, error.to_string()));
        }
    }

    problems
}

pub fn validate_install_location(input: &str) -> Result<String> {
    if input.is_empty() {
        bail!("默认安装位置不能为空");
    }
    if input.encode_utf16().count() > MAX_INSTALL_PATH_UTF16 {
        bail!("默认安装位置过长");
    }
    let expanded = expand_environment(input)?;
    let normalized = normalize_absolute(&expanded)?;
    reject_protected_locations(&normalized)?;
    Ok(normalized)
}

pub fn validate_source_image_path(input: &str) -> Result<()> {
    if input.chars().any(|ch| ch == '\0' || (ch as u32) < 0x20) {
        bail!("图片路径包含控制字符");
    }
    let lower = input.to_ascii_lowercase();
    if ["http://", "https://", "ftp://", "file://"]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        bail!("图片路径必须是本机文件，不能是 URL");
    }
    if input.starts_with(r"\\") {
        bail!("图片路径不能是 UNC 或设备路径");
    }
    Ok(())
}

pub fn is_special_background(value: &str) -> bool {
    [
        "acrylic",
        "acrylicblur",
        "mica",
        "windowsmanaged",
        "transparent",
    ]
    .iter()
    .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

pub fn is_hex_color(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(|byte| byte.is_ascii_hexdigit())
}

pub fn recommended_install_location(name: &str) -> String {
    format!(r"%LocalAppData%\Programs\{}", safe_file_stem(name, "App"))
}

pub fn safe_file_stem(name: &str, fallback: &str) -> String {
    let mut output: String = name
        .trim()
        .chars()
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control()
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    while output.ends_with('.') || output.ends_with(' ') {
        output.pop();
    }
    if output.is_empty() {
        fallback.to_string()
    } else {
        output
    }
}

pub fn resolve_existing_file(base_dir: &Path, value: &str) -> Result<PathBuf> {
    if value.trim().is_empty() {
        bail!("路径为空");
    }
    let value = value.trim();
    validate_source_image_path(value)?;
    let path = PathBuf::from(value);
    let path = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };
    let metadata = std::fs::symlink_metadata(&path)
        .with_context(|| format!("文件不存在：{}", path.display()))?;
    if metadata.file_attributes_reparse_point() {
        bail!("路径不能是重解析点：{}", path.display());
    }
    if !metadata.is_file() {
        bail!("路径不是普通文件：{}", path.display());
    }
    Ok(std::fs::canonicalize(&path).unwrap_or(path))
}

trait ReparseMetadata {
    fn file_attributes_reparse_point(&self) -> bool;
}

impl ReparseMetadata for std::fs::Metadata {
    fn file_attributes_reparse_point(&self) -> bool {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
            self.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

fn decode(bytes: &[u8]) -> Result<String> {
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        bail!("config.toml is larger than 64 KiB");
    }
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bail!("config.toml must be UTF-8 without a BOM");
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .context("config.toml is not valid UTF-8")
}

fn optional_string(value: Option<&Value>, field: &str) -> Result<String> {
    match value {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => bail!("{field} must be a string"),
        None => Ok(String::new()),
    }
}

fn set_optional(document: &mut DocumentMut, key: &str, value_text: &str) {
    if value_text.is_empty() {
        document.remove(key);
    } else {
        document[key] = value(value_text);
    }
}

fn problem(field: ConfigField, message: impl Into<String>) -> ConfigProblem {
    ConfigProblem {
        field,
        message: message.into(),
    }
}

fn expand_environment(input: &str) -> Result<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '%' {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index] != '%' {
                index += 1;
            }
            if index == chars.len() {
                bail!("环境变量引用未闭合：{input:?}");
            }
            let name: String = chars[start + 1..index].iter().collect();
            if name.is_empty() {
                bail!("环境变量引用为空：{input:?}");
            }
            let value =
                std::env::var_os(&name).with_context(|| format!("环境变量 %{name}% 未定义"))?;
            output.push_str(&value.to_string_lossy());
            index += 1;
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }
    Ok(output)
}

fn normalize_absolute(path: &str) -> Result<String> {
    if path.encode_utf16().count() > MAX_INSTALL_PATH_UTF16 {
        bail!("默认安装位置过长");
    }
    for ch in path.chars() {
        if ch == '\0' || (ch as u32) < 0x20 {
            bail!("默认安装位置包含控制字符");
        }
    }
    let bytes = path.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || (bytes[2] != b'\\' && bytes[2] != b'/')
    {
        bail!("默认安装位置必须是本地绝对盘符路径：{path:?}");
    }
    let rest = &path[2..];
    if rest.contains(':') {
        bail!("默认安装位置不能包含备用数据流");
    }
    let mut components = Vec::new();
    for component in rest.split(['\\', '/']) {
        if component.is_empty() {
            continue;
        }
        if component == "." || component == ".." {
            bail!("默认安装位置不能包含 . 或 ..");
        }
        if component.ends_with('.') || component.ends_with(' ') {
            bail!("路径组件不能以点或空格结尾：{component:?}");
        }
        if is_device_name(component) {
            bail!("路径包含 Windows 保留设备名：{component:?}");
        }
        components.push(component);
    }
    let drive = bytes[0].to_ascii_uppercase() as char;
    if components.is_empty() {
        Ok(format!("{drive}:\\"))
    } else {
        Ok(format!("{drive}:\\{}", components.join("\\")))
    }
}

fn is_device_name(component: &str) -> bool {
    let stem = component
        .split_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(component);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit()
            && upper.as_bytes()[3] != b'0')
}

fn reject_protected_locations(path: &str) -> Result<()> {
    let bytes = path.as_bytes();
    if bytes.len() == 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
        bail!("默认安装位置不能是盘符根目录：{path:?}");
    }

    let mut roots: Vec<(String, bool, &'static str)> = Vec::new();
    if let Some(windows) = env_path("SystemRoot").or_else(|| env_path("WINDIR")) {
        roots.push((windows, true, "Windows 目录"));
    }
    if let Some(program_files) = env_path("ProgramFiles") {
        roots.push((program_files, false, "Program Files 根目录"));
    }
    if let Some(program_files_x86) = env_path("ProgramFiles(x86)") {
        roots.push((program_files_x86, false, "Program Files (x86) 根目录"));
    }
    if let Some(program_data) = env_path("ProgramData") {
        roots.push((program_data, false, "ProgramData 根目录"));
    }
    if let Some(profile) = env_path("USERPROFILE") {
        roots.push((format!("{profile}\\Desktop"), false, "桌面"));
        roots.push((profile, false, "用户主目录"));
    }
    if let Some(app_data) = env_path("APPDATA") {
        roots.push((
            format!("{app_data}\\Microsoft\\Windows\\Start Menu"),
            false,
            "开始菜单根目录",
        ));
    }

    for (root, include_children, description) in roots {
        if same_or_under(path, &root, include_children) {
            bail!("默认安装位置不能是{description}：{path:?}");
        }
    }
    Ok(())
}

fn env_path(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

fn same_or_under(path: &str, root: &str, include_children: bool) -> bool {
    let path = normalize_for_compare(path);
    let root = normalize_for_compare(root);
    if path == root {
        return true;
    }
    include_children
        && path
            .strip_prefix(&root)
            .is_some_and(|rest| rest.starts_with('\\'))
}

fn normalize_for_compare(path: &str) -> String {
    path.trim_end_matches(['\\', '/']).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aip_config() {
        let source = r#"
protocol = 1
name = "Example App"
icon = 'C:\icons\app.ico'
background = "acrylic"

[defaults]
installLocation = 'D:\Apps\Example App'
createStartMenuFolder = true
createDesktopShortcut = false
"#;
        let draft = parse(source).unwrap();
        assert_eq!(draft.name, "Example App");
        assert!(draft.create_start_menu_folder);
        assert!(!draft.create_desktop_shortcut);
    }

    #[test]
    fn render_preserves_comments_and_unknown_keys() {
        let source = r#"# keep me
protocol = 1
name = "Old"
future = { enabled = true }

[defaults]
installLocation = 'D:\Apps\Old'
createStartMenuFolder = true
createDesktopShortcut = true
"#;
        let draft = ConfigDraft {
            name: "New".into(),
            install_location: r"D:\Apps\New".into(),
            ..ConfigDraft::default()
        };
        let rendered = render(&draft, Some(source)).unwrap();
        assert!(rendered.contains("# keep me"));
        assert!(rendered.contains("future = { enabled = true }"));
        assert!(rendered.contains("name = \"New\""));
        assert!(!rendered.contains("Old"));
    }

    #[test]
    fn background_values_follow_aip() {
        assert!(is_special_background("AcrylicBlur"));
        assert!(is_hex_color("#1E90FF"));
        assert!(!is_hex_color("#fff"));
        assert!(!is_hex_color("#GGGGGG"));
    }

    #[test]
    fn recommends_a_safe_install_path() {
        assert_eq!(
            recommended_install_location("My: App"),
            r"%LocalAppData%\Programs\My_ App"
        );
        assert_eq!(safe_file_stem("...", "App"), "App");
    }

    #[test]
    fn rejects_relative_install_paths() {
        assert!(validate_install_location("app/sub").is_err());
        assert!(validate_install_location(r"C:\Apps\..\Windows").is_err());
        assert!(validate_install_location(r"C:\Apps\file:stream").is_err());
    }
}
