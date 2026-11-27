//! Shared application vocabulary and UI state types.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Step {
    #[default]
    App,
    Defaults,
    Market,
    Inputs,
    Package,
    Settings,
}

impl Step {
    pub const ALL: [Step; 5] = [
        Step::App,
        Step::Defaults,
        Step::Market,
        Step::Inputs,
        Step::Package,
    ];

    pub fn index(self) -> usize {
        match self {
            Step::App => 0,
            Step::Defaults => 1,
            Step::Market => 2,
            Step::Inputs => 3,
            Step::Package => 4,
            Step::Settings => 5,
        }
    }

    pub fn number(self) -> &'static str {
        match self {
            Step::App => "01",
            Step::Defaults => "02",
            Step::Market => "03",
            Step::Inputs => "04",
            Step::Package => "05",
            Step::Settings => "S",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Step::App => "应用信息",
            Step::Defaults => "安装默认值",
            Step::Market => "组件市场",
            Step::Inputs => "构建输入",
            Step::Package => "签名与打包",
            Step::Settings => "设置",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Step::App => "身份与外观",
            Step::Defaults => "安装行为",
            Step::Market => "官方与社区",
            Step::Inputs => "组件与载荷",
            Step::Package => "证书与输出",
            Step::Settings => "网络与自动检测",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Step::App => "编辑 AIP v1 的应用名称、图标与安装界面背景。",
            Step::Defaults => "设置安装器首次打开时的默认安装行为。",
            Step::Market => "从 jsDelivr 目录浏览客户端、后端与 stub，并按 SHA-256 安全下载。",
            Step::Inputs => "将 stub、client、backend、payload 与客户端资源整理进构建目录。",
            Step::Package => "选择签名方式、配置 packager，并输出最终单文件安装器。",
            Step::Settings => "管理 GitHub 加速、出口 IP 自动检测与本机组件定位。",
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            Step::App => Some(Step::Defaults),
            Step::Defaults => Some(Step::Market),
            Step::Market => Some(Step::Inputs),
            Step::Inputs => Some(Step::Package),
            Step::Package => None,
            Step::Settings => None,
        }
    }

    pub fn previous(self) -> Option<Self> {
        match self {
            Step::App => None,
            Step::Defaults => Some(Step::App),
            Step::Market => Some(Step::Defaults),
            Step::Inputs => Some(Step::Market),
            Step::Package => Some(Step::Inputs),
            Step::Settings => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundMode {
    #[default]
    Default,
    Image,
    Color,
    Material,
}

impl BackgroundMode {
    pub const ALL: [BackgroundMode; 4] = [
        BackgroundMode::Default,
        BackgroundMode::Image,
        BackgroundMode::Color,
        BackgroundMode::Material,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BackgroundMode::Default => "默认白色",
            BackgroundMode::Image => "图片",
            BackgroundMode::Color => "纯色",
            BackgroundMode::Material => "系统材质",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundMaterial {
    #[default]
    Acrylic,
    AcrylicBlur,
    Mica,
    WindowsManaged,
    Transparent,
}

impl BackgroundMaterial {
    pub const ALL: [BackgroundMaterial; 5] = [
        BackgroundMaterial::Acrylic,
        BackgroundMaterial::AcrylicBlur,
        BackgroundMaterial::Mica,
        BackgroundMaterial::WindowsManaged,
        BackgroundMaterial::Transparent,
    ];

    pub fn value(self) -> &'static str {
        match self {
            BackgroundMaterial::Acrylic => "acrylic",
            BackgroundMaterial::AcrylicBlur => "acrylicBlur",
            BackgroundMaterial::Mica => "mica",
            BackgroundMaterial::WindowsManaged => "windowsManaged",
            BackgroundMaterial::Transparent => "transparent",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BackgroundMaterial::Acrylic => "Acrylic",
            BackgroundMaterial::AcrylicBlur => "Acrylic + Blur",
            BackgroundMaterial::Mica => "Mica",
            BackgroundMaterial::WindowsManaged => "Windows Managed",
            BackgroundMaterial::Transparent => "透明",
        }
    }

    pub fn from_value(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|material| material.value().eq_ignore_ascii_case(value))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignMode {
    #[default]
    Unsigned,
    Thumbprint,
    Pfx,
}

impl SignMode {
    pub const ALL: [SignMode; 3] = [SignMode::Unsigned, SignMode::Thumbprint, SignMode::Pfx];

    pub fn label(self) -> &'static str {
        match self {
            SignMode::Unsigned => "开发未签名",
            SignMode::Thumbprint => "证书存储",
            SignMode::Pfx => "PFX 文件",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            SignMode::Unsigned => "仅用于本地测试，AIP 发布包不得使用。",
            SignMode::Thumbprint => "使用当前用户证书存储中的 SHA-1 指纹。",
            SignMode::Pfx => "从 PFX 文件签名，密码通过环境变量传入。",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RegionStatus {
    #[default]
    Unknown,
    Checking,
    Domestic {
        country_code: String,
        country: String,
    },
    Overseas {
        country_code: String,
        country: String,
    },
    Failed(String),
}

impl RegionStatus {
    pub fn label(&self) -> String {
        match self {
            RegionStatus::Unknown => "出口 IP 尚未检测".into(),
            RegionStatus::Checking => "正在检测出口 IP...".into(),
            RegionStatus::Domestic {
                country_code,
                country,
            } => format!("国内出口 · {country} ({country_code})"),
            RegionStatus::Overseas {
                country_code,
                country,
            } => format!("海外出口 · {country} ({country_code})"),
            RegionStatus::Failed(error) => format!("检测失败：{error}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiField {
    Name,
    Icon,
    BackgroundImage,
    BackgroundColor,
    InstallLocation,
    BuildDir,
    Stub,
    Client,
    Backend,
    Payload,
    ClientAssets,
    Packager,
    Output,
    Pfx,
    PfxPasswordEnv,
    Thumbprint,
    TimestampUrl,
    Signtool,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub field: UiField,
    pub step: Step,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BuildPhase {
    #[default]
    Idle,
    Preparing,
    Running,
    Success,
    Failure,
}

impl BuildPhase {
    pub fn is_busy(self) -> bool {
        matches!(self, BuildPhase::Preparing | BuildPhase::Running)
    }

    pub fn label(self) -> &'static str {
        match self {
            BuildPhase::Idle => "等待构建",
            BuildPhase::Preparing => "准备输入",
            BuildPhase::Running => "正在打包",
            BuildPhase::Success => "构建成功",
            BuildPhase::Failure => "构建失败",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Notice {
    pub text: String,
    pub tone: NoticeTone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathTarget {
    LoadConfig,
    SaveConfigAs,
    BuildDir,
    Stub,
    Client,
    Backend,
    Payload,
    ClientAssets,
    Packager,
    Output,
    Icon,
    Background,
    Pfx,
    Signtool,
}

pub struct DetectedWorkspace {
    pub root: PathBuf,
    pub packager: PathBuf,
    pub stub: PathBuf,
    pub client: PathBuf,
    pub backend: PathBuf,
    pub client_assets: PathBuf,
    pub build_dir: PathBuf,
}

impl DetectedWorkspace {
    pub fn found_count(&self) -> usize {
        [
            &self.packager,
            &self.stub,
            &self.client,
            &self.backend,
            &self.client_assets,
        ]
        .into_iter()
        .filter(|path| path.exists())
        .count()
    }
}
