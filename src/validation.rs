//! GUI-side AIP configuration and build-input validation.
//!
//! These checks are intentionally fast feedback for the author. The packager
//! remains the authoritative validator and security boundary.

use std::path::{Path, PathBuf};

use crate::app::State;
use crate::config_file::{self, ConfigField};
use crate::model::{BackgroundMode, Issue, SignMode, Step, UiField};

pub fn config_issues(state: &State) -> Vec<Issue> {
    config_file::validate(&state.config_draft())
        .into_iter()
        .map(|problem| match problem.field {
            ConfigField::Name => Issue {
                field: UiField::Name,
                step: Step::App,
                message: problem.message,
            },
            ConfigField::Icon => Issue {
                field: UiField::Icon,
                step: Step::App,
                message: problem.message,
            },
            ConfigField::Background => Issue {
                field: match state.background_mode {
                    BackgroundMode::Color => UiField::BackgroundColor,
                    _ => UiField::BackgroundImage,
                },
                step: Step::App,
                message: problem.message,
            },
            ConfigField::InstallLocation => Issue {
                field: UiField::InstallLocation,
                step: Step::Defaults,
                message: problem.message,
            },
        })
        .collect()
}

pub fn build_issues(state: &State) -> Vec<Issue> {
    let mut issues = config_issues(state);
    let build_dir = path_from_edit(&state.build_dir_edit);

    if state.build_dir_edit.text.trim().is_empty() {
        push(
            &mut issues,
            UiField::BuildDir,
            Step::Inputs,
            "请选择构建目录",
        );
    } else if build_dir.exists() && !build_dir.is_dir() {
        push(
            &mut issues,
            UiField::BuildDir,
            Step::Inputs,
            "构建路径不是目录",
        );
    }

    require_file(
        &mut issues,
        UiField::Stub,
        Step::Inputs,
        &state.stub_edit,
        "stub.exe",
    );
    require_file(
        &mut issues,
        UiField::Client,
        Step::Inputs,
        &state.client_edit,
        "client.exe",
    );
    require_file(
        &mut issues,
        UiField::Backend,
        Step::Inputs,
        &state.backend_edit,
        "backend.exe",
    );
    require_file(
        &mut issues,
        UiField::Payload,
        Step::Inputs,
        &state.payload_edit,
        "payload.zip",
    );

    if !state.client_assets_edit.text.trim().is_empty()
        && !path_from_edit(&state.client_assets_edit).is_dir()
    {
        push(
            &mut issues,
            UiField::ClientAssets,
            Step::Inputs,
            "客户端资源路径不是目录",
        );
    }

    require_file(
        &mut issues,
        UiField::Packager,
        Step::Package,
        &state.packager_edit,
        "packager.exe",
    );
    validate_output(state, &build_dir, &mut issues);
    validate_signing(state, &mut issues);
    validate_media_files(state, &build_dir, &mut issues);
    issues
}

fn validate_output(state: &State, build_dir: &Path, issues: &mut Vec<Issue>) {
    if state.output_edit.text.trim().is_empty() {
        push(issues, UiField::Output, Step::Package, "请选择输出文件");
        return;
    }

    let output = path_from_edit(&state.output_edit);
    if !output
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
    {
        push(
            issues,
            UiField::Output,
            Step::Package,
            "输出文件必须使用 .exe 扩展名",
        );
    }
    if output.is_dir() {
        push(issues, UiField::Output, Step::Package, "输出路径不能是目录");
    }
    if output.exists() && !state.force_overwrite {
        push(
            issues,
            UiField::Output,
            Step::Package,
            "输出文件已存在；请启用覆盖选项",
        );
    }
    if !state.build_dir_edit.text.trim().is_empty()
        && is_path_under(&resolve_path(&output), &resolve_path(build_dir))
    {
        push(
            issues,
            UiField::Output,
            Step::Package,
            "输出文件必须位于构建目录之外",
        );
    }
}

fn validate_signing(state: &State, issues: &mut Vec<Issue>) {
    match state.sign_mode {
        SignMode::Unsigned => {}
        SignMode::Thumbprint => {
            let thumbprint = state.thumbprint_edit.text.trim();
            if thumbprint.len() != 40 || !thumbprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                push(
                    issues,
                    UiField::Thumbprint,
                    Step::Package,
                    "证书指纹必须是 40 个十六进制字符",
                );
            }
        }
        SignMode::Pfx => {
            require_file(
                issues,
                UiField::Pfx,
                Step::Package,
                &state.pfx_edit,
                "PFX 证书",
            );
            let env_name = state.pfx_password_env_edit.text.trim();
            if env_name.is_empty() {
                push(
                    issues,
                    UiField::PfxPasswordEnv,
                    Step::Package,
                    "请填写保存 PFX 密码的环境变量名",
                );
            } else if std::env::var_os(env_name).is_none() {
                push(
                    issues,
                    UiField::PfxPasswordEnv,
                    Step::Package,
                    format!("环境变量 {env_name} 尚未设置"),
                );
            }
        }
    }

    if state.sign_mode != SignMode::Unsigned {
        let timestamp = state.timestamp_url_edit.text.trim();
        if timestamp.is_empty()
            || !(timestamp.starts_with("http://") || timestamp.starts_with("https://"))
        {
            push(
                issues,
                UiField::TimestampUrl,
                Step::Package,
                "时间戳地址必须是 http:// 或 https:// URL",
            );
        }
    }

    if !state.signtool_edit.text.trim().is_empty()
        && !path_from_edit(&state.signtool_edit).is_file()
    {
        push(
            issues,
            UiField::Signtool,
            Step::Package,
            "signtool.exe 路径不存在",
        );
    }
}

fn validate_media_files(state: &State, build_dir: &Path, issues: &mut Vec<Issue>) {
    if !state.icon_edit.text.trim().is_empty() {
        if let Err(error) =
            config_file::resolve_existing_file(build_dir, state.icon_edit.text.trim())
        {
            push(
                issues,
                UiField::Icon,
                Step::App,
                format!("图标文件无效：{error}"),
            );
        }
    }

    if state.background_mode == BackgroundMode::Image
        && !state.background_image_edit.text.trim().is_empty()
    {
        if let Err(error) =
            config_file::resolve_existing_file(build_dir, state.background_image_edit.text.trim())
        {
            push(
                issues,
                UiField::BackgroundImage,
                Step::App,
                format!("背景图片无效：{error}"),
            );
        }
    }
}

fn require_file(
    issues: &mut Vec<Issue>,
    field: UiField,
    step: Step,
    edit: &rune::TextEdit,
    description: &str,
) {
    if edit.text.trim().is_empty() {
        push(issues, field, step, format!("请选择{description}"));
    } else if !path_from_edit(edit).is_file() {
        push(
            issues,
            field,
            step,
            format!("{description} 不存在或不是文件"),
        );
    }
}

fn push(issues: &mut Vec<Issue>, field: UiField, step: Step, message: impl Into<String>) {
    issues.push(Issue {
        field,
        step,
        message: message.into(),
    });
}

pub fn path_from_edit(edit: &rune::TextEdit) -> PathBuf {
    PathBuf::from(edit.text.trim())
}

pub fn optional_path_from_edit(edit: &rune::TextEdit) -> Option<PathBuf> {
    let text = edit.text.trim();
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}

pub fn resolve_path(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn is_path_under(path: &Path, root: &Path) -> bool {
    let path = path.to_string_lossy().replace('/', "\\");
    let mut root = root.to_string_lossy().replace('/', "\\");
    while root.ends_with('\\') {
        root.pop();
    }
    #[cfg(windows)]
    {
        let path = path.to_ascii_lowercase();
        let root = root.to_ascii_lowercase();
        path == root || path.starts_with(&(root + "\\"))
    }
    #[cfg(not(windows))]
    {
        path == root || path.starts_with(&(root + "/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
