//! The packager subprocess boundary and build workspace preparation.
//!
//! The GUI treats `packager.exe` as a stable CLI service. This keeps the AIP
//! validation authority in one place while the GUI remains responsible for the
//! authoring experience and for assembling the packager's expected input layout.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use anyhow::{bail, Context, Result};
use rune::Waker;
use walkdir::WalkDir;

use crate::config_file::{self, ConfigDraft};

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub build_dir: PathBuf,
    pub stub: PathBuf,
    pub client: PathBuf,
    pub backend: PathBuf,
    pub payload: PathBuf,
    pub client_assets: Option<PathBuf>,
    pub packager: PathBuf,
    pub output: PathBuf,
    pub force: bool,
    pub config: ConfigDraft,
    pub config_base: Option<String>,
    pub signing: SigningOptions,
    pub timestamp_url: String,
    pub signtool: Option<PathBuf>,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SigningOptions {
    Unsigned,
    Thumbprint(String),
    Pfx { path: PathBuf, password_env: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Tool,
    Error,
}

#[derive(Clone, Debug)]
pub struct LogEvent {
    pub kind: LogKind,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct BuildFinished {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub output: PathBuf,
}

#[derive(Clone, Debug)]
pub enum BuildEvent {
    Log(LogEvent),
    Finished(BuildFinished),
}

pub fn start(options: BuildOptions, wake: Waker) -> Receiver<BuildEvent> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || run_worker(options, sender, wake));
    receiver
}

fn run_worker(options: BuildOptions, sender: Sender<BuildEvent>, wake: Waker) {
    info(&sender, &wake, "正在整理构建目录并同步输入文件...");

    let output_absolute = absolute(&options.output).unwrap_or_else(|_| options.output.clone());
    let finish = |success, exit_code| {
        send(
            &sender,
            &wake,
            BuildEvent::Finished(BuildFinished {
                success,
                exit_code,
                output: output_absolute.clone(),
            }),
        );
    };

    if let Err(error) = prepare_workspace(&options) {
        emit_error(&sender, &wake, format!("构建准备失败：{error:#}"));
        finish(false, None);
        return;
    }
    info(&sender, &wake, "构建目录已就绪，开始调用 packager.exe。");

    let mut command = Command::new(&options.packager);
    command
        .args(build_arguments(&options))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            emit_error(
                &sender,
                &wake,
                format!(
                    "无法启动 packager：{}（{error}）",
                    options.packager.display()
                ),
            );
            finish(false, None);
            return;
        }
    };

    let stdout_handle = child.stdout.take().map(|stdout| {
        let sender = sender.clone();
        let wake = wake.clone();
        thread::spawn(move || read_lines(stdout, LogKind::Tool, sender, wake))
    });
    let stderr_handle = child.stderr.take().map(|stderr| {
        let sender = sender.clone();
        let wake = wake.clone();
        thread::spawn(move || read_lines(stderr, LogKind::Error, sender, wake))
    });

    match child.wait() {
        Ok(status) => {
            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }
            if status.success() {
                info(
                    &sender,
                    &wake,
                    format!("打包完成：{}", output_absolute.display()),
                );
            } else {
                emit_error(
                    &sender,
                    &wake,
                    format!("packager 退出码：{}", status.code().unwrap_or(-1)),
                );
            }
            finish(status.success(), status.code());
        }
        Err(error) => {
            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }
            emit_error(&sender, &wake, format!("等待 packager 失败：{error}"));
            finish(false, None);
        }
    }
}

pub fn prepare_workspace(options: &BuildOptions) -> Result<()> {
    let build_dir = absolute(&options.build_dir)?;
    let packager = absolute_existing_file(&options.packager, "packager.exe")?;
    let _ = packager;

    if let SigningOptions::Pfx { path, password_env } = &options.signing {
        let pfx = absolute_existing_file(path, "PFX 证书")?;
        if is_path_under(&pfx, &build_dir) {
            bail!("PFX 证书不能放在构建目录内");
        }
        if password_env.trim().is_empty() {
            bail!("PFX 模式需要填写密码环境变量名");
        }
        if std::env::var_os(password_env).is_none() {
            bail!("环境变量 {password_env} 未设置，无法读取 PFX 密码");
        }
    }

    std::fs::create_dir_all(&build_dir)
        .with_context(|| format!("无法创建构建目录：{}", build_dir.display()))?;
    if !build_dir.is_dir() {
        bail!("构建路径不是目录：{}", build_dir.display());
    }

    sync_file(&options.stub, &build_dir.join("stub.exe"), "stub.exe")?;
    sync_file(&options.client, &build_dir.join("client.exe"), "client.exe")?;
    sync_file(
        &options.backend,
        &build_dir.join("backend.exe"),
        "backend.exe",
    )?;
    sync_file(
        &options.payload,
        &build_dir.join("payload.zip"),
        "payload.zip",
    )?;

    if let Some(assets) = &options.client_assets {
        sync_client_assets(assets, &build_dir.join("assets"))?;
    }

    let config_text = config_file::render(&options.config, options.config_base.as_deref())?;
    std::fs::write(build_dir.join("config.toml"), config_text.as_bytes())
        .context("无法写入构建目录中的 config.toml")?;

    let output = absolute(&options.output)?;
    let output_parent = output.parent().context("输出路径缺少父目录")?;
    std::fs::create_dir_all(output_parent)
        .with_context(|| format!("无法创建输出目录：{}", output_parent.display()))?;

    if is_path_under(&output, &build_dir) {
        bail!("输出文件必须位于构建目录之外：{}", output.display());
    }
    if output.exists() && !options.force {
        bail!("输出文件已存在；启用“覆盖已有输出”后才能替换");
    }

    Ok(())
}

pub fn build_arguments(options: &BuildOptions) -> Vec<String> {
    let mut args = vec![
        "build".to_string(),
        "--build-dir".to_string(),
        options.build_dir.to_string_lossy().into_owned(),
        "--output".to_string(),
        options.output.to_string_lossy().into_owned(),
    ];
    if options.force {
        args.push("--force".to_string());
    }
    match &options.signing {
        SigningOptions::Unsigned => args.push("--unsigned".to_string()),
        SigningOptions::Thumbprint(value) => {
            args.push("--thumbprint".to_string());
            args.push(value.clone());
        }
        SigningOptions::Pfx { path, password_env } => {
            args.push("--pfx".to_string());
            args.push(path.to_string_lossy().into_owned());
            args.push("--pfx-password-env".to_string());
            args.push(password_env.clone());
        }
    }
    if !options.timestamp_url.trim().is_empty() {
        args.push("--timestamp-url".to_string());
        args.push(options.timestamp_url.trim().to_string());
    }
    if let Some(signtool) = &options.signtool {
        if !signtool.as_os_str().is_empty() {
            args.push("--signtool".to_string());
            args.push(signtool.to_string_lossy().into_owned());
        }
    }
    if !options.description.trim().is_empty() {
        args.push("--description".to_string());
        args.push(options.description.trim().to_string());
    }
    args
}

fn read_lines(reader: impl std::io::Read, kind: LogKind, sender: Sender<BuildEvent>, wake: Waker) {
    for line in BufReader::new(reader).lines() {
        match line {
            Ok(line) => send(
                &sender,
                &wake,
                BuildEvent::Log(LogEvent { kind, text: line }),
            ),
            Err(error) => {
                send(
                    &sender,
                    &wake,
                    BuildEvent::Log(LogEvent {
                        kind: LogKind::Error,
                        text: format!("读取 packager 输出失败：{error}"),
                    }),
                );
                break;
            }
        }
    }
}

fn sync_file(source: &Path, destination: &Path, description: &str) -> Result<()> {
    let source = absolute_existing_file(source, description)?;
    if same_path_if_exists(&source, destination) {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("无法创建目录：{}", parent.display()))?;
    }
    std::fs::copy(&source, destination).with_context(|| {
        format!(
            "无法同步 {description}：{} -> {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn sync_client_assets(source: &Path, destination: &Path) -> Result<()> {
    let source = absolute(source)?;
    let metadata = std::fs::symlink_metadata(&source)
        .with_context(|| format!("客户端资源目录不存在：{}", source.display()))?;
    if is_reparse_point(&metadata) {
        bail!("客户端资源目录不能是重解析点：{}", source.display());
    }
    if !metadata.is_dir() {
        bail!("客户端资源路径不是目录：{}", source.display());
    }

    std::fs::create_dir_all(destination)
        .with_context(|| format!("无法创建资源目录：{}", destination.display()))?;

    for entry in WalkDir::new(&source).follow_links(false) {
        let entry = entry.with_context(|| format!("无法读取资源目录：{}", source.display()))?;
        let metadata = std::fs::symlink_metadata(entry.path())
            .with_context(|| format!("无法读取资源元数据：{}", entry.path().display()))?;
        if is_reparse_point(&metadata) {
            bail!("客户端资源中不能包含重解析点：{}", entry.path().display());
        }
        if metadata.is_dir() {
            continue;
        }
        if !metadata.is_file() {
            bail!("客户端资源只能包含普通文件：{}", entry.path().display());
        }
        let relative = entry
            .path()
            .strip_prefix(&source)
            .with_context(|| format!("资源路径越界：{}", entry.path().display()))?;
        let target = destination.join(relative);
        if same_path_if_exists(entry.path(), &target) {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建资源目录：{}", parent.display()))?;
        }
        std::fs::copy(entry.path(), &target).with_context(|| {
            format!(
                "无法同步客户端资源：{} -> {}",
                entry.path().display(),
                target.display()
            )
        })?;
    }
    Ok(())
}

fn absolute_existing_file(path: &Path, description: &str) -> Result<PathBuf> {
    let path = absolute(path)?;
    let metadata = std::fs::symlink_metadata(&path)
        .with_context(|| format!("{description} 不存在：{}", path.display()))?;
    if is_reparse_point(&metadata) {
        bail!("{description} 不能是重解析点：{}", path.display());
    }
    if !metadata.is_file() {
        bail!("{description} 不是普通文件：{}", path.display());
    }
    std::fs::canonicalize(&path)
        .with_context(|| format!("无法解析 {description}：{}", path.display()))
}

fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).with_context(|| format!("无法解析路径：{}", path.display()))
}

fn same_path_if_exists(left: &Path, right: &Path) -> bool {
    let Ok(left) = std::fs::canonicalize(left) else {
        return false;
    };
    let Ok(right) = std::fs::canonicalize(right) else {
        return false;
    };
    paths_equal(&left, &right)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn is_path_under(path: &Path, root: &Path) -> bool {
    let path_text = path.to_string_lossy().replace('/', "\\");
    let root_text = root.to_string_lossy().replace('/', "\\");
    #[cfg(windows)]
    {
        let path_text = path_text.to_ascii_lowercase();
        let mut root_text = root_text.to_ascii_lowercase();
        while root_text.ends_with('\\') {
            root_text.pop();
        }
        path_text == root_text || path_text.starts_with(&(root_text + "\\"))
    }
    #[cfg(not(windows))]
    {
        path_text == root_text || path_text.starts_with(&(root_text + "/"))
    }
}

fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

fn info(sender: &Sender<BuildEvent>, wake: &Waker, text: impl Into<String>) {
    log(sender, wake, LogKind::Info, text.into());
}

fn emit_error(sender: &Sender<BuildEvent>, wake: &Waker, text: impl Into<String>) {
    log(sender, wake, LogKind::Error, text.into());
}

fn log(sender: &Sender<BuildEvent>, wake: &Waker, kind: LogKind, text: String) {
    send(sender, wake, BuildEvent::Log(LogEvent { kind, text }));
}

fn send(sender: &Sender<BuildEvent>, wake: &Waker, event: BuildEvent) {
    if sender.send(event).is_ok() {
        wake();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(signing: SigningOptions) -> BuildOptions {
        BuildOptions {
            build_dir: PathBuf::from(r"D:\work\build"),
            stub: PathBuf::from(r"D:\src\stub.exe"),
            client: PathBuf::from(r"D:\src\client.exe"),
            backend: PathBuf::from(r"D:\src\backend.exe"),
            payload: PathBuf::from(r"D:\src\payload.zip"),
            client_assets: None,
            packager: PathBuf::from(r"D:\tools\packager.exe"),
            output: PathBuf::from(r"D:\dist\setup.exe"),
            force: true,
            config: ConfigDraft::default(),
            config_base: None,
            signing,
            timestamp_url: "http://timestamp.digicert.com".into(),
            signtool: None,
            description: "Example".into(),
        }
    }

    #[test]
    fn unsigned_arguments_are_one_shot_and_complete() {
        let args = build_arguments(&options(SigningOptions::Unsigned));
        assert_eq!(args[0], "build");
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--build-dir", r"D:\work\build"]));
        assert!(args.contains(&"--force".to_string()));
        assert!(args.contains(&"--unsigned".to_string()));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--description", "Example"]));
    }

    #[test]
    fn pfx_arguments_never_embed_the_password() {
        let args = build_arguments(&options(SigningOptions::Pfx {
            path: PathBuf::from(r"D:\keys\release.pfx"),
            password_env: "AIP_PFX_PASSWORD".into(),
        }));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--pfx", r"D:\keys\release.pfx"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--pfx-password-env", "AIP_PFX_PASSWORD"]));
        assert!(!args
            .iter()
            .any(|arg| arg.to_ascii_lowercase().contains("password=")));
    }

    #[test]
    fn prepares_the_packager_input_layout() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let assets = temp.path().join("client-assets");
        std::fs::create_dir_all(assets.join("nested")).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("stub.exe"), b"stub").unwrap();
        std::fs::write(source.join("client.exe"), b"client").unwrap();
        std::fs::write(source.join("backend.exe"), b"backend").unwrap();
        std::fs::write(source.join("payload.zip"), b"payload").unwrap();
        std::fs::write(source.join("packager.exe"), b"packager").unwrap();
        std::fs::write(assets.join("runtime.dll"), b"runtime").unwrap();
        std::fs::write(assets.join("nested/data.bin"), b"data").unwrap();

        let build_dir = temp.path().join("build");
        let output = temp.path().join("dist/setup.exe");
        let mut request = options(SigningOptions::Unsigned);
        request.build_dir = build_dir.clone();
        request.stub = source.join("stub.exe");
        request.client = source.join("client.exe");
        request.backend = source.join("backend.exe");
        request.payload = source.join("payload.zip");
        request.packager = source.join("packager.exe");
        request.output = output;
        request.client_assets = Some(assets);
        request.force = false;
        request.config = ConfigDraft {
            name: "Prepared App".into(),
            install_location: r"D:\Apps\Prepared App".into(),
            create_start_menu_folder: true,
            create_desktop_shortcut: false,
            ..ConfigDraft::default()
        };

        prepare_workspace(&request).unwrap();
        assert_eq!(std::fs::read(build_dir.join("stub.exe")).unwrap(), b"stub");
        assert_eq!(
            std::fs::read(build_dir.join("payload.zip")).unwrap(),
            b"payload"
        );
        assert_eq!(
            std::fs::read(build_dir.join("assets/nested/data.bin")).unwrap(),
            b"data"
        );
        let config = std::fs::read_to_string(build_dir.join("config.toml")).unwrap();
        assert!(config.contains("name = \"Prepared App\""));
        assert!(config.contains("createDesktopShortcut = false"));
        assert!(request.output.parent().unwrap().is_dir());
    }

    #[test]
    fn path_containment_uses_components() {
        assert!(is_path_under(
            Path::new(r"C:\Build\foo.exe"),
            Path::new(r"C:\Build")
        ));
        assert!(!is_path_under(
            Path::new(r"C:\Builder\foo.exe"),
            Path::new(r"C:\Build")
        ));
    }
}
