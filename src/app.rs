//! Application state and user actions for the packager GUI.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use anyhow::{Context, Result};
use rune::{FileDialogOptions, TextEdit, Tx, Waker};

use crate::build_job::{
    self, BuildEvent, BuildFinished, BuildOptions, LogEvent, LogKind, SigningOptions,
};
use crate::config_file::{self, ConfigDraft, LoadedConfig};
use crate::detect;
use crate::i18n::Language;
use crate::market::{self, Catalog, Component, InstalledComponent, MarketEvent, RegionInfo};
use crate::model::{
    BackgroundMaterial, BackgroundMode, BuildPhase, Notice, NoticeTone, PathTarget, RegionStatus,
    SignMode, Step, UiField,
};
use crate::settings::{self, GithubAccelerationSettings, GuiSettings};
use crate::validation;

pub const DEFAULT_TIMESTAMP_URL: &str = "http://timestamp.digicert.com";

#[derive(Clone, Debug)]
pub struct MarketDownload {
    pub component_id: String,
    pub message: String,
    pub received: u64,
    pub total: Option<u64>,
}

pub struct State {
    pub step: Step,
    pub language: Language,
    pub name_edit: TextEdit,
    pub icon_edit: TextEdit,
    pub background_mode: BackgroundMode,
    pub background_material: BackgroundMaterial,
    pub background_image_edit: TextEdit,
    pub background_color_edit: TextEdit,
    pub install_location_edit: TextEdit,
    pub create_start_menu_folder: bool,
    pub create_desktop_shortcut: bool,

    pub build_dir_edit: TextEdit,
    pub stub_edit: TextEdit,
    pub client_edit: TextEdit,
    pub backend_edit: TextEdit,
    pub payload_edit: TextEdit,
    pub client_assets_edit: TextEdit,

    pub packager_edit: TextEdit,
    pub output_edit: TextEdit,
    pub sign_mode: SignMode,
    pub pfx_edit: TextEdit,
    pub pfx_password_env_edit: TextEdit,
    pub thumbprint_edit: TextEdit,
    pub timestamp_url_edit: TextEdit,
    pub signtool_edit: TextEdit,
    pub description_edit: TextEdit,
    pub force_overwrite: bool,

    pub config_base: Option<String>,
    pub config_file_path: Option<PathBuf>,
    pub dirty: bool,
    pub issues: Vec<crate::model::Issue>,
    pub notice: Option<Notice>,
    pub autodetect_note: String,

    pub phase: BuildPhase,
    pub logs: Vec<LogEvent>,
    pub build_receiver: Option<Receiver<BuildEvent>>,
    pub spinner_time: f64,
    pub last_output: Option<PathBuf>,
    pub last_exit_code: Option<i32>,

    pub catalog: Option<Catalog>,
    pub catalog_loading: bool,
    pub catalog_error: Option<String>,
    pub region_status: RegionStatus,
    pub accelerator_enabled: bool,
    pub accelerator_prefix_edit: TextEdit,
    pub accelerator_error: Option<String>,
    pub market_sender: Option<Sender<MarketEvent>>,
    pub market_receiver: Option<Receiver<MarketEvent>>,
    pub market_download: Option<MarketDownload>,
    pub installed_market: Vec<InstalledComponent>,

    pub waker: Option<Waker>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            step: Step::App,
            language: Language::default(),
            name_edit: TextEdit::new("Example App"),
            icon_edit: TextEdit::default(),
            background_mode: BackgroundMode::Default,
            background_material: BackgroundMaterial::Acrylic,
            background_image_edit: TextEdit::default(),
            background_color_edit: TextEdit::new("#1E90FF"),
            install_location_edit: TextEdit::new(config_file::recommended_install_location(
                "Example App",
            )),
            create_start_menu_folder: true,
            create_desktop_shortcut: true,

            build_dir_edit: TextEdit::default(),
            stub_edit: TextEdit::default(),
            client_edit: TextEdit::default(),
            backend_edit: TextEdit::default(),
            payload_edit: TextEdit::default(),
            client_assets_edit: TextEdit::default(),

            packager_edit: TextEdit::default(),
            output_edit: TextEdit::new(r"target\packager-output\Example App Setup.exe"),
            sign_mode: SignMode::Unsigned,
            pfx_edit: TextEdit::default(),
            pfx_password_env_edit: TextEdit::new("AIP_PFX_PASSWORD"),
            thumbprint_edit: TextEdit::default(),
            timestamp_url_edit: TextEdit::new(DEFAULT_TIMESTAMP_URL),
            signtool_edit: TextEdit::default(),
            description_edit: TextEdit::new("Anyinstaller package for Example App"),
            force_overwrite: false,

            config_base: None,
            config_file_path: None,
            dirty: false,
            issues: Vec::new(),
            notice: Some(Notice {
                text: "编辑配置后即可一键整理输入并调用 packager。".into(),
                tone: NoticeTone::Info,
            }),
            autodetect_note: "尚未自动定位工作区".into(),

            phase: BuildPhase::Idle,
            logs: vec![LogEvent {
                kind: LogKind::Info,
                text: "构建日志将在这里实时显示。".into(),
            }],
            build_receiver: None,
            spinner_time: 0.0,
            last_output: None,
            last_exit_code: None,

            catalog: None,
            catalog_loading: false,
            catalog_error: None,
            region_status: RegionStatus::Unknown,
            accelerator_enabled: false,
            accelerator_prefix_edit: TextEdit::new(market::DEFAULT_GITHUB_ACCELERATOR),
            accelerator_error: None,
            market_sender: None,
            market_receiver: None,
            market_download: None,
            installed_market: Vec::new(),

            waker: None,
        }
    }
}

pub fn init() -> State {
    let mut state = State::default();
    let saved = settings::load();
    state.language = saved.language;
    state.accelerator_enabled = saved.github_acceleration.enabled;
    if !saved.github_acceleration.prefix.trim().is_empty() {
        state
            .accelerator_prefix_edit
            .replace_text(saved.github_acceleration.prefix);
    }
    state.auto_detect(None);
    state
}

pub fn setup(state: &mut State, waker: Waker) {
    state.waker = Some(waker);
    if !rune::is_headless() {
        state.start_initial_market_tasks();
    }
}

pub fn poll(state: &mut State, now: f64, tx: &mut Tx) -> bool {
    state.spinner_time = now;
    let mut changed = false;
    let mut finished = None;
    let mut disconnected = false;

    if let Some(receiver) = state.build_receiver.as_ref() {
        loop {
            match receiver.try_recv() {
                Ok(BuildEvent::Log(event)) => {
                    state.logs.push(event);
                    if state.phase == BuildPhase::Preparing {
                        state.phase = BuildPhase::Running;
                    }
                    changed = true;
                }
                Ok(BuildEvent::Finished(event)) => {
                    finished = Some(event);
                    changed = true;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
    }

    if let Some(event) = finished {
        apply_finished(state, event);
        state.build_receiver = None;
        changed = true;
    } else if disconnected {
        state.phase = BuildPhase::Failure;
        state.build_receiver = None;
        state.notice = Some(Notice {
            text: "构建线程意外结束，请查看日志。".into(),
            tone: NoticeTone::Error,
        });
        changed = true;
    }

    let mut market_events = Vec::new();
    if let Some(receiver) = state.market_receiver.as_ref() {
        loop {
            match receiver.try_recv() {
                Ok(event) => {
                    market_events.push(event);
                    changed = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }
    for event in market_events {
        apply_market_event(state, event, tx);
    }

    if state.logs.len() > 300 {
        let remove = state.logs.len() - 300;
        state.logs.drain(0..remove);
        changed = true;
    }

    if state.phase.is_busy() {
        tx.wake_after(Duration::from_millis(33));
        changed = true;
    }
    changed
}

fn apply_market_event(state: &mut State, event: MarketEvent, tx: &mut Tx) {
    match event {
        MarketEvent::CatalogLoaded(catalog) => {
            state.installed_market = catalog
                .components
                .iter()
                .filter_map(market::installed_component)
                .collect();
            state.catalog = Some(catalog);
            state.catalog_loading = false;
            state.catalog_error = None;
            let count = state
                .catalog
                .as_ref()
                .map_or(0, |catalog| catalog.components.len());
            state.notify(
                format!("市场目录已从 jsDelivr 加载，共 {count} 个组件。"),
                NoticeTone::Success,
            );
        }
        MarketEvent::CatalogFailed(error) => {
            state.catalog_loading = false;
            state.catalog_error = Some(error.clone());
            state.notify(format!("市场目录加载失败：{error}"), NoticeTone::Error);
        }
        MarketEvent::RegionLoaded(region) => {
            apply_region_info(state, region);
        }
        MarketEvent::DownloadMessage {
            component_id,
            message,
        } => {
            if state
                .market_download
                .as_ref()
                .is_some_and(|download| download.component_id == component_id)
            {
                if let Some(download) = state.market_download.as_mut() {
                    download.message = message;
                }
            }
        }
        MarketEvent::DownloadProgress {
            component_id,
            received,
            total,
        } => {
            if let Some(download) = state.market_download.as_mut() {
                if download.component_id == component_id {
                    download.received = received;
                    download.total = total;
                }
            }
        }
        MarketEvent::DownloadInstalled(installed) => {
            let installed = *installed;
            state.market_download = None;
            state
                .installed_market
                .retain(|existing| existing.component.id != installed.component.id);
            state.installed_market.push(installed.clone());
            state.apply_installed_component(installed, tx);
        }
        MarketEvent::DownloadFailed {
            component_id,
            message,
        } => {
            if state
                .market_download
                .as_ref()
                .is_some_and(|download| download.component_id == component_id)
            {
                state.market_download = None;
            }
            state.notify(format!("市场组件下载失败：{message}"), NoticeTone::Error);
        }
    }
}

fn apply_region_info(state: &mut State, region: RegionInfo) {
    state.region_status = if region.domestic {
        RegionStatus::Domestic {
            ip: region.ip.clone(),
            country_code: region.country_code.clone(),
            country: region.country.clone(),
            fallback: region.fallback,
        }
    } else {
        RegionStatus::Overseas {
            ip: region.ip.clone(),
            country_code: region.country_code.clone(),
            country: region.country.clone(),
        }
    };
    state.accelerator_enabled = region.domestic;
    let _ = state.persist_acceleration_settings();
    state.notify(
        if region.domestic {
            if region.fallback {
                "无法访问 ipwho.is，已按国内出口处理并自动启用加速。".into()
            } else {
                format!("检测到国内出口 {}，已自动启用 GitHub 加速。", region.ip)
            }
        } else {
            format!("检测到海外出口 {}，已自动关闭 GitHub 加速。", region.ip)
        },
        NoticeTone::Info,
    );
}

fn apply_finished(state: &mut State, event: BuildFinished) {
    state.last_output = Some(event.output.clone());
    state.last_exit_code = event.exit_code;
    if event.success {
        state.phase = BuildPhase::Success;
        state.notice = Some(Notice {
            text: format!("安装器已生成：{}", event.output.display()),
            tone: NoticeTone::Success,
        });
    } else {
        state.phase = BuildPhase::Failure;
        state.notice = Some(Notice {
            text: "打包失败，请根据日志修正输入或签名配置。".into(),
            tone: NoticeTone::Error,
        });
    }
}

impl State {
    pub fn config_draft(&self) -> ConfigDraft {
        ConfigDraft {
            name: self.name_edit.text.trim().to_string(),
            icon: self.icon_edit.text.trim().to_string(),
            background: match self.background_mode {
                BackgroundMode::Default => String::new(),
                BackgroundMode::Image => self.background_image_edit.text.trim().to_string(),
                BackgroundMode::Color => self.background_color_edit.text.trim().to_string(),
                BackgroundMode::Material => self.background_material.value().to_string(),
            },
            install_location: self.install_location_edit.text.trim().to_string(),
            create_start_menu_folder: self.create_start_menu_folder,
            create_desktop_shortcut: self.create_desktop_shortcut,
        }
    }

    pub fn signing_options(&self) -> SigningOptions {
        match self.sign_mode {
            SignMode::Unsigned => SigningOptions::Unsigned,
            SignMode::Thumbprint => {
                SigningOptions::Thumbprint(self.thumbprint_edit.text.trim().to_string())
            }
            SignMode::Pfx => SigningOptions::Pfx {
                path: validation::path_from_edit(&self.pfx_edit),
                password_env: self.pfx_password_env_edit.text.trim().to_string(),
            },
        }
    }

    pub fn build_options(&self) -> BuildOptions {
        BuildOptions {
            build_dir: validation::path_from_edit(&self.build_dir_edit),
            stub: validation::path_from_edit(&self.stub_edit),
            client: validation::path_from_edit(&self.client_edit),
            backend: validation::path_from_edit(&self.backend_edit),
            payload: validation::path_from_edit(&self.payload_edit),
            client_assets: validation::optional_path_from_edit(&self.client_assets_edit),
            packager: validation::path_from_edit(&self.packager_edit),
            output: validation::path_from_edit(&self.output_edit),
            force: self.force_overwrite,
            config: self.config_draft(),
            config_base: self.render_base(),
            signing: self.signing_options(),
            timestamp_url: self.timestamp_url_edit.text.trim().to_string(),
            signtool: validation::optional_path_from_edit(&self.signtool_edit),
            description: self.description_edit.text.trim().to_string(),
        }
    }

    pub fn render_base(&self) -> Option<String> {
        self.config_base.clone().or_else(|| {
            let path = validation::path_from_edit(&self.build_dir_edit).join("config.toml");
            std::fs::read_to_string(path).ok()
        })
    }

    pub fn issue(&self, field: UiField) -> Option<&crate::model::Issue> {
        self.issues.iter().find(|issue| issue.field == field)
    }

    pub fn step_issue_count(&self, step: Step) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.step == step)
            .count()
    }

    pub fn mark_dirty(&mut self, tx: &mut Tx) {
        self.dirty = true;
        self.issues.clear();
        tx.set_document_edited(true);
    }

    pub fn notify(&mut self, text: impl Into<String>, tone: NoticeTone) {
        self.notice = Some(Notice {
            text: text.into(),
            tone,
        });
    }

    pub fn set_step(&mut self, step: Step) {
        self.step = step;
    }

    pub fn set_background_mode(&mut self, mode: BackgroundMode, tx: &mut Tx) {
        if self.background_mode != mode {
            self.background_mode = mode;
            self.mark_dirty(tx);
        }
    }

    pub fn set_background_material(&mut self, material: BackgroundMaterial, tx: &mut Tx) {
        if self.background_material != material {
            self.background_material = material;
            self.mark_dirty(tx);
        }
    }

    pub fn use_recommended_install_location(&mut self, tx: &mut Tx) {
        let path = config_file::recommended_install_location(&self.name_edit.text);
        self.install_location_edit.replace_text(path);
        self.mark_dirty(tx);
    }

    fn ensure_market_channel(&mut self) -> Sender<MarketEvent> {
        if let Some(sender) = &self.market_sender {
            return sender.clone();
        }
        let (sender, receiver) = mpsc::channel();
        self.market_sender = Some(sender.clone());
        self.market_receiver = Some(receiver);
        sender
    }

    fn start_initial_market_tasks(&mut self) {
        if self.market_sender.is_some() {
            return;
        }
        let Some(waker) = self.waker.clone() else {
            return;
        };
        let sender = self.ensure_market_channel();
        self.catalog_loading = true;
        self.region_status = RegionStatus::Checking;
        market::start_catalog_refresh(sender.clone(), waker.clone());
        market::start_region_check(sender, waker);
    }

    pub fn refresh_catalog(&mut self) {
        let Some(waker) = self.waker.clone() else {
            self.catalog_error = Some("Rune 运行时尚未就绪".into());
            return;
        };
        let sender = self.ensure_market_channel();
        self.catalog_loading = true;
        self.catalog_error = None;
        market::start_catalog_refresh(sender, waker);
    }

    pub fn check_region(&mut self) {
        let Some(waker) = self.waker.clone() else {
            self.region_status = RegionStatus::Domestic {
                ip: "不可用".into(),
                country_code: "CN".into(),
                country: "Rune 运行时尚未就绪，默认国内".into(),
                fallback: true,
            };
            self.accelerator_enabled = true;
            return;
        };
        let sender = self.ensure_market_channel();
        self.region_status = RegionStatus::Checking;
        market::start_region_check(sender, waker);
    }

    pub fn acceleration_settings(&self) -> GithubAccelerationSettings {
        GithubAccelerationSettings {
            enabled: self.accelerator_enabled,
            prefix: self.accelerator_prefix_edit.text.trim().to_string(),
        }
    }

    pub fn set_language(&mut self, language: Language, _tx: &mut Tx) {
        if self.language == language {
            return;
        }
        self.language = language;
        if self.persist_acceleration_settings() {
            self.notify(
                crate::i18n::tr_lang("界面语言已切换。", language),
                NoticeTone::Success,
            );
        }
    }

    pub fn toggle_acceleration(&mut self, _tx: &mut Tx) {
        let enable = !self.accelerator_enabled;
        if enable {
            match market::normalize_accelerator_prefix(&self.accelerator_prefix_edit.text) {
                Ok(prefix) => {
                    self.accelerator_prefix_edit.replace_text(prefix);
                    self.accelerator_error = None;
                }
                Err(error) => {
                    self.accelerator_error = Some(error.to_string());
                    self.notify(error.to_string(), NoticeTone::Error);
                    return;
                }
            }
        }
        self.accelerator_enabled = enable;
        let _ = self.persist_acceleration_settings();
    }

    pub fn save_acceleration(&mut self, _tx: &mut Tx) {
        let _ = self.persist_acceleration_settings();
    }

    fn persist_acceleration_settings(&mut self) -> bool {
        match market::normalize_accelerator_prefix(&self.accelerator_prefix_edit.text) {
            Ok(prefix) => {
                self.accelerator_prefix_edit.replace_text(prefix);
                self.accelerator_error = None;
            }
            Err(error) => {
                self.accelerator_error = Some(error.to_string());
                self.notify(error.to_string(), NoticeTone::Error);
                return false;
            }
        }
        let settings = GuiSettings {
            language: self.language,
            github_acceleration: self.acceleration_settings(),
        };
        match settings::save(&settings) {
            Ok(path) => {
                self.notify(
                    format!("GitHub 加速设置已保存：{}", path.display()),
                    NoticeTone::Success,
                );
                true
            }
            Err(error) => {
                self.accelerator_error = Some(format!("{error:#}"));
                self.notify(format!("保存加速设置失败：{error:#}"), NoticeTone::Error);
                false
            }
        }
    }

    pub fn start_market_download(&mut self, component: Component, _tx: &mut Tx) {
        if self.market_download.is_some() {
            self.notify("已有组件正在下载，请稍候。", NoticeTone::Warning);
            return;
        }
        let Some(waker) = self.waker.clone() else {
            self.notify("Rune 运行时尚未就绪", NoticeTone::Error);
            return;
        };
        let sender = self.ensure_market_channel();
        if self.accelerator_enabled {
            if let Err(error) =
                market::normalize_accelerator_prefix(&self.accelerator_prefix_edit.text)
            {
                self.accelerator_error = Some(error.to_string());
                self.notify(error.to_string(), NoticeTone::Error);
                return;
            }
        }
        self.market_download = Some(MarketDownload {
            component_id: component.id.clone(),
            message: "准备下载...".into(),
            received: 0,
            total: None,
        });
        self.notify(
            format!("开始下载 {}（最新 Release）...", component.name),
            NoticeTone::Info,
        );
        market::start_download(component, self.acceleration_settings(), sender, waker);
    }

    pub fn installed_component(&self, id: &str) -> Option<&InstalledComponent> {
        self.installed_market
            .iter()
            .find(|installed| installed.component.id == id)
    }

    pub fn use_installed_component(&mut self, id: &str, tx: &mut Tx) {
        let Some(installed) = self.installed_component(id).cloned() else {
            self.notify("没有可用的市场缓存", NoticeTone::Warning);
            return;
        };
        self.apply_installed_component(installed, tx);
    }

    fn apply_installed_component(&mut self, installed: InstalledComponent, tx: &mut Tx) {
        match installed.component.kind {
            market::ComponentKind::Client => {
                set_edit_path(&mut self.client_edit, &installed.executable);
                if let Some(assets) = &installed.assets_dir {
                    set_edit_path(&mut self.client_assets_edit, assets);
                }
            }
            market::ComponentKind::Backend => {
                set_edit_path(&mut self.backend_edit, &installed.executable);
            }
            market::ComponentKind::Stub => {
                set_edit_path(&mut self.stub_edit, &installed.executable);
            }
        }
        self.mark_dirty(tx);
        self.notify(
            format!("已应用市场组件：{}", installed.component.name),
            NoticeTone::Success,
        );
    }

    pub fn open_settings(&mut self) {
        self.step = Step::Settings;
        if matches!(self.region_status, RegionStatus::Unknown) {
            self.check_region();
        }
        if self.catalog.is_none() && !self.catalog_loading {
            self.refresh_catalog();
        }
    }

    pub fn run_auto_detection(&mut self, tx: &mut Tx) {
        self.auto_detect(Some(tx));
        self.check_region();
        self.refresh_catalog();
        self.notify(
            "已启动工作区、出口 IP 与市场目录自动检测。",
            NoticeTone::Info,
        );
    }

    pub fn auto_detect(&mut self, tx: Option<&mut Tx>) {
        let Some(detected) = detect::find_workspace() else {
            self.autodetect_note = "未找到 Anyinstaller 工作区，请手动选择输入文件。".into();
            if let Some(tx) = tx {
                self.mark_dirty(tx);
            }
            return;
        };

        set_if_present(&mut self.packager_edit, &detected.packager);
        set_if_present(&mut self.stub_edit, &detected.stub);
        set_if_present(&mut self.client_edit, &detected.client);
        set_if_present(&mut self.backend_edit, &detected.backend);
        set_if_present(&mut self.client_assets_edit, &detected.client_assets);
        set_edit_path(&mut self.build_dir_edit, &detected.build_dir);
        let output_name = format!(
            "{} Setup.exe",
            config_file::safe_file_stem(&self.name_edit.text, "Installer")
        );
        let output = detected
            .build_dir
            .parent()
            .unwrap_or(Path::new("."))
            .join("packager-output")
            .join(output_name);
        set_edit_path(&mut self.output_edit, &output);
        self.autodetect_note = format!("已定位工作区：{}", detected.root.display());
        if let Some(tx) = tx {
            self.mark_dirty(tx);
            self.notify(
                format!("已自动定位 {} 个构建输入。", detected.found_count()),
                NoticeTone::Success,
            );
        }
    }

    pub fn save_config(&mut self, tx: &mut Tx) {
        self.issues = validation::config_issues(self);
        if let Some(issue) = self.issues.first() {
            self.step = issue.step;
            self.notify(issue.message.clone(), NoticeTone::Error);
            return;
        }
        let build_dir = validation::path_from_edit(&self.build_dir_edit);
        if self.build_dir_edit.text.trim().is_empty() {
            self.browse(tx, PathTarget::SaveConfigAs);
            return;
        }
        match self.write_config_to(&build_dir.join("config.toml")) {
            Ok(path) => {
                tx.set_document_edited(false);
                self.notify(
                    format!("配置已保存：{}", path.display()),
                    NoticeTone::Success,
                );
            }
            Err(error) => self.notify(format!("保存配置失败：{error:#}"), NoticeTone::Error),
        }
    }

    pub fn write_config_to(&mut self, path: &Path) -> Result<PathBuf> {
        let rendered = config_file::render(&self.config_draft(), self.render_base().as_deref())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建目录：{}", parent.display()))?;
        }
        std::fs::write(path, rendered.as_bytes())
            .with_context(|| format!("无法写入 {}", path.display()))?;
        self.config_base = Some(rendered);
        self.config_file_path =
            Some(std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()));
        self.dirty = false;
        Ok(path.to_path_buf())
    }

    pub fn load_config_from(&mut self, path: &Path, tx: &mut Tx) -> Result<()> {
        let loaded = config_file::load(path)?;
        self.apply_loaded_config(loaded, path);
        tx.set_document_edited(false);
        self.notify(
            format!("已载入 AIP 配置：{}", path.display()),
            NoticeTone::Success,
        );
        Ok(())
    }

    fn apply_loaded_config(&mut self, loaded: LoadedConfig, path: &Path) {
        let LoadedConfig { draft, source } = loaded;
        let config_dir = path.parent().unwrap_or(Path::new("."));
        let icon = rebase_media_path(&draft.icon, config_dir);
        let background = rebase_media_path(&draft.background, config_dir);
        self.name_edit.replace_text(draft.name);
        self.icon_edit.replace_text(icon);
        self.install_location_edit
            .replace_text(draft.install_location);
        self.create_start_menu_folder = draft.create_start_menu_folder;
        self.create_desktop_shortcut = draft.create_desktop_shortcut;

        if background.is_empty() {
            self.background_mode = BackgroundMode::Default;
        } else if config_file::is_hex_color(&background) {
            self.background_mode = BackgroundMode::Color;
            self.background_color_edit.replace_text(background);
        } else if let Some(material) = BackgroundMaterial::from_value(&background) {
            self.background_mode = BackgroundMode::Material;
            self.background_material = material;
        } else {
            self.background_mode = BackgroundMode::Image;
            self.background_image_edit.replace_text(background);
        }

        self.config_base = Some(source);
        self.config_file_path = Some(path.to_path_buf());
        self.dirty = false;
        self.issues.clear();
    }

    pub fn validate_config_only(&mut self) -> bool {
        self.issues = validation::config_issues(self);
        if let Some(issue) = self.issues.first() {
            self.step = issue.step;
            self.notify(issue.message.clone(), NoticeTone::Error);
            false
        } else {
            self.notify("配置通过 AIP v1 基础校验。", NoticeTone::Success);
            true
        }
    }

    pub fn start_build(&mut self, tx: &mut Tx) {
        if self.phase.is_busy() {
            self.notify("当前已有打包任务在运行。", NoticeTone::Warning);
            return;
        }
        self.issues = validation::build_issues(self);
        if let Some(issue) = self.issues.first() {
            self.step = issue.step;
            self.logs.push(LogEvent {
                kind: LogKind::Error,
                text: format!("输入校验失败：{}", issue.message),
            });
            self.notify(
                format!("发现 {} 个问题，已定位到第一项。", self.issues.len()),
                NoticeTone::Error,
            );
            return;
        }

        let wake = self
            .waker
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(|| {}));
        let receiver = build_job::start(self.build_options(), wake);
        self.build_receiver = Some(receiver);
        self.phase = BuildPhase::Preparing;
        self.logs.clear();
        self.logs.push(LogEvent {
            kind: LogKind::Info,
            text: "构建任务已启动。".into(),
        });
        self.last_output = None;
        self.last_exit_code = None;
        self.notify("正在准备构建目录...", NoticeTone::Info);
        self.step = Step::Package;
        tx.set_document_edited(self.dirty);
    }

    pub fn browse(&mut self, tx: &mut Tx, target: PathTarget) {
        let start = self.dialog_directory(target);
        let mut options = FileDialogOptions {
            title: crate::i18n::tr(picker_title(target)),
            directory: start,
            file_name: None,
            filters: Vec::new(),
        };
        match target {
            PathTarget::LoadConfig => {
                options.file_name = Some("config.toml".into());
                options = options.filter(crate::i18n::tr("AIP 配置"), &["toml"]);
            }
            PathTarget::SaveConfigAs => {
                options.file_name = Some("config.toml".into());
                options = options.filter("AIP config", &["toml"]);
            }
            PathTarget::Stub | PathTarget::Client | PathTarget::Backend | PathTarget::Packager => {
                options = options.filter(crate::i18n::tr("Windows 可执行文件"), &["exe"]);
            }
            PathTarget::Payload => options = options.filter(crate::i18n::tr("ZIP 载荷"), &["zip"]),
            PathTarget::Output => {
                options.file_name = Some("Setup.exe".into());
                options = options.filter("Windows executable", &["exe"]);
            }
            PathTarget::Icon => {
                options = options.filter(
                    crate::i18n::tr("图标"),
                    &["ico", "png", "webp", "avif", "jpg", "jpeg"],
                );
            }
            PathTarget::Background => {
                options = options.filter(
                    crate::i18n::tr("背景图片"),
                    &["png", "webp", "avif", "jpg", "jpeg"],
                );
            }
            PathTarget::Pfx => options = options.filter(crate::i18n::tr("PFX 证书"), &["pfx"]),
            PathTarget::Signtool => {
                options.file_name = Some("signtool.exe".into());
                options = options.filter("Windows executable", &["exe"]);
            }
            PathTarget::BuildDir | PathTarget::ClientAssets | PathTarget::ImportCustomComponent => {
            }
        }

        match target {
            PathTarget::BuildDir | PathTarget::ClientAssets | PathTarget::ImportCustomComponent => {
                tx.open_folder(options, move |state: &mut State, result, tx| {
                    answer_picker(state, target, result, tx);
                });
            }
            PathTarget::SaveConfigAs | PathTarget::Output => {
                tx.save_file(options, move |state: &mut State, result, tx| {
                    answer_picker(state, target, result, tx);
                });
            }
            _ => {
                tx.open_file(options, move |state: &mut State, result, tx| {
                    answer_picker(state, target, result, tx);
                });
            }
        }
    }

    fn import_custom_component(&mut self, path: &Path, tx: &mut Tx) -> Result<()> {
        if !path.is_dir() {
            anyhow::bail!("请选择包含运行组件的文件夹");
        }
        let client = path.join("client.exe");
        let backend = path.join("backend.exe");
        let stub = path.join("stub.exe");
        let assets = path.join("assets");
        let mut imported = Vec::new();
        if client.is_file() {
            set_edit_path(&mut self.client_edit, &client);
            if assets.is_dir() {
                set_edit_path(&mut self.client_assets_edit, &assets);
            }
            imported.push("client");
        }
        if backend.is_file() {
            set_edit_path(&mut self.backend_edit, &backend);
            imported.push("backend");
        }
        if stub.is_file() {
            set_edit_path(&mut self.stub_edit, &stub);
            imported.push("stub");
        }
        if imported.is_empty() {
            anyhow::bail!("目录中没有 client.exe、backend.exe 或 stub.exe");
        }
        self.mark_dirty(tx);
        self.notify(
            format!("已导入自定义组件：{}", imported.join("、")),
            NoticeTone::Success,
        );
        Ok(())
    }

    fn accept_path(&mut self, target: PathTarget, path: &Path, tx: &mut Tx) -> Result<()> {
        match target {
            PathTarget::LoadConfig => {
                self.load_config_from(path, tx)?;
                return Ok(());
            }
            PathTarget::ImportCustomComponent => {
                self.import_custom_component(path, tx)?;
                return Ok(());
            }
            PathTarget::SaveConfigAs => {
                self.issues = validation::config_issues(self);
                if let Some(issue) = self.issues.first() {
                    self.notify(issue.message.clone(), NoticeTone::Error);
                    return Ok(());
                }
                let path = if path.extension().is_some() {
                    path.to_path_buf()
                } else {
                    path.with_extension("toml")
                };
                let saved = self.write_config_to(&path)?;
                tx.set_document_edited(false);
                self.notify(
                    format!("配置已导出：{}", saved.display()),
                    NoticeTone::Success,
                );
                return Ok(());
            }
            PathTarget::BuildDir => set_edit_path(&mut self.build_dir_edit, path),
            PathTarget::Stub => set_edit_path(&mut self.stub_edit, path),
            PathTarget::Client => set_edit_path(&mut self.client_edit, path),
            PathTarget::Backend => set_edit_path(&mut self.backend_edit, path),
            PathTarget::Payload => set_edit_path(&mut self.payload_edit, path),
            PathTarget::ClientAssets => set_edit_path(&mut self.client_assets_edit, path),
            PathTarget::Packager => set_edit_path(&mut self.packager_edit, path),
            PathTarget::Output => set_edit_path(&mut self.output_edit, path),
            PathTarget::Icon => set_edit_path(&mut self.icon_edit, path),
            PathTarget::Background => {
                set_edit_path(&mut self.background_image_edit, path);
                self.background_mode = BackgroundMode::Image;
            }
            PathTarget::Pfx => set_edit_path(&mut self.pfx_edit, path),
            PathTarget::Signtool => set_edit_path(&mut self.signtool_edit, path),
        }
        self.mark_dirty(tx);
        Ok(())
    }

    fn dialog_directory(&self, target: PathTarget) -> Option<PathBuf> {
        let current = match target {
            PathTarget::LoadConfig
            | PathTarget::SaveConfigAs
            | PathTarget::BuildDir
            | PathTarget::ImportCustomComponent => validation::path_from_edit(&self.build_dir_edit),
            PathTarget::Stub => validation::path_from_edit(&self.stub_edit),
            PathTarget::Client => validation::path_from_edit(&self.client_edit),
            PathTarget::Backend => validation::path_from_edit(&self.backend_edit),
            PathTarget::Payload => validation::path_from_edit(&self.payload_edit),
            PathTarget::ClientAssets => validation::path_from_edit(&self.client_assets_edit),
            PathTarget::Packager => validation::path_from_edit(&self.packager_edit),
            PathTarget::Output => validation::path_from_edit(&self.output_edit),
            PathTarget::Icon => validation::path_from_edit(&self.icon_edit),
            PathTarget::Background => validation::path_from_edit(&self.background_image_edit),
            PathTarget::Pfx => validation::path_from_edit(&self.pfx_edit),
            PathTarget::Signtool => validation::path_from_edit(&self.signtool_edit),
        };
        if current.is_dir() {
            Some(current)
        } else {
            current
                .parent()
                .filter(|path| path.is_dir())
                .map(Path::to_path_buf)
        }
    }
}

fn answer_picker(
    state: &mut State,
    target: PathTarget,
    result: Result<Option<PathBuf>, rune::FileDialogError>,
    tx: &mut Tx,
) {
    match result {
        Ok(Some(path)) => {
            if let Err(error) = state.accept_path(target, &path, tx) {
                state.notify(format!("{error:#}"), NoticeTone::Error);
            }
        }
        Ok(None) => {}
        Err(error) => state.notify(error.to_string(), NoticeTone::Error),
    }
}

fn rebase_media_path(value: &str, config_dir: &Path) -> String {
    let value = value.trim();
    if value.is_empty()
        || config_file::is_special_background(value)
        || config_file::is_hex_color(value)
    {
        return value.to_string();
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return value.to_string();
    }
    let joined = config_dir.join(path);
    std::path::absolute(&joined)
        .unwrap_or(joined)
        .to_string_lossy()
        .into_owned()
}

fn set_edit_path(edit: &mut TextEdit, path: &Path) {
    edit.replace_text(path.to_string_lossy().into_owned());
}

fn set_if_present(edit: &mut TextEdit, path: &Path) {
    if path.exists() {
        set_edit_path(edit, path);
    }
}

fn picker_title(target: PathTarget) -> &'static str {
    match target {
        PathTarget::LoadConfig => "载入 AIP 配置",
        PathTarget::SaveConfigAs => "导出 AIP 配置",
        PathTarget::BuildDir => "选择构建目录",
        PathTarget::Stub => "选择 stub.exe",
        PathTarget::Client => "选择 client.exe",
        PathTarget::Backend => "选择 backend.exe",
        PathTarget::Payload => "选择 payload.zip",
        PathTarget::ClientAssets => "选择客户端 assets 目录",
        PathTarget::ImportCustomComponent => "导入自定义运行组件目录",
        PathTarget::Packager => "选择 packager.exe",
        PathTarget::Output => "选择安装器输出位置",
        PathTarget::Icon => "选择应用图标",
        PathTarget::Background => "选择背景图片",
        PathTarget::Pfx => "选择 PFX 签名证书",
        PathTarget::Signtool => "选择 signtool.exe",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_mode_maps_to_aip_values() {
        let mut state = State {
            background_mode: BackgroundMode::Material,
            background_material: BackgroundMaterial::Mica,
            ..State::default()
        };
        assert_eq!(state.config_draft().background, "mica");

        state.background_mode = BackgroundMode::Color;
        state.background_color_edit.replace_text("#112233");
        assert_eq!(state.config_draft().background, "#112233");
    }

    #[test]
    fn default_state_has_valid_install_template() {
        let state = State::default();
        assert!(
            config_file::validate_install_location(state.install_location_edit.text.trim()).is_ok()
        );
    }

    #[test]
    fn loaded_relative_media_paths_are_rebased_to_the_config_directory() {
        let value = rebase_media_path("assets/icon.ico", Path::new(r"D:\Project"));
        assert!(value.starts_with(r"D:\Project"));
        assert!(value.ends_with("assets\\icon.ico"));
        assert_eq!(rebase_media_path("mica", Path::new(r"D:\Project")), "mica");
        assert_eq!(
            rebase_media_path("#112233", Path::new(r"D:\Project")),
            "#112233"
        );
    }

    #[test]
    fn step_navigation_is_symmetric() {
        for step in Step::ALL {
            if let Some(next) = step.next() {
                assert_eq!(next.previous(), Some(step));
            }
            if let Some(previous) = step.previous() {
                assert_eq!(previous.next(), Some(step));
            }
        }
    }
}
