//! Best-effort discovery of the sibling Anyinstaller repositories.

use std::path::{Path, PathBuf};

use crate::model::DetectedWorkspace;

pub fn find_workspace() -> Option<DetectedWorkspace> {
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let current_dir = std::env::current_dir().ok();
    let mut starts = Vec::new();
    if let Some(dir) = current_dir {
        starts.push(dir);
    }
    if let Some(dir) = executable_dir {
        if !starts.contains(&dir) {
            starts.push(dir);
        }
    }

    for start in starts {
        for ancestor in start.ancestors().take(8) {
            if ancestor.join("packager").join("Cargo.toml").is_file()
                && ancestor.join("stub").join("Cargo.toml").is_file()
            {
                return Some(from_workspace_root(ancestor));
            }
        }
    }

    let executable_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let packager = executable_dir.join("packager.exe");
    if packager.is_file() {
        return Some(DetectedWorkspace {
            root: executable_dir.clone(),
            packager,
            stub: executable_dir.join("stub.exe"),
            client: executable_dir.join("client.exe"),
            backend: executable_dir.join("backend.exe"),
            client_assets: executable_dir.join("assets"),
            build_dir: executable_dir.join("build"),
        });
    }
    None
}

fn from_workspace_root(root: &Path) -> DetectedWorkspace {
    let root = root.to_path_buf();
    let packager = first_file(&[
        root.join(r"packager\target\release\packager.exe"),
        root.join(r"packager\target\debug\packager.exe"),
    ])
    .unwrap_or_else(|| root.join(r"packager\target\release\packager.exe"));
    let stub = first_file(&[
        root.join(r"stub\target\release\stub.exe"),
        root.join(r"stub\target\debug\stub.exe"),
    ])
    .unwrap_or_else(|| root.join(r"stub\target\release\stub.exe"));
    let client = first_file(&[
        root.join(r"voidclient\dist\client\client.exe"),
        root.join(r"voidclient\build\client\client.exe"),
    ])
    .unwrap_or_else(|| root.join(r"voidclient\dist\client\client.exe"));
    let backend = first_file(&[
        root.join(r"voidbackend\target\release\backend.exe"),
        root.join(r"voidbackend\target\debug\backend.exe"),
    ])
    .unwrap_or_else(|| root.join(r"voidbackend\target\release\backend.exe"));
    let client_assets = [
        root.join(r"voidclient\dist\client\assets"),
        root.join(r"voidclient\build\client\assets"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
    .unwrap_or_else(|| root.join(r"voidclient\dist\client\assets"));
    let build_dir = root.join(r"anyinstaller-packager-gui\target\gui-build");

    DetectedWorkspace {
        root,
        packager,
        stub,
        client,
        backend,
        client_assets,
        build_dir,
    }
}

fn first_file(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.is_file()).cloned()
}
