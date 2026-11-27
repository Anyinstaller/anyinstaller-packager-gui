# Anyinstaller Packager GUI

基于 [Rune](https://github.com/anomalyco/rune) 的 Anyinstaller AIP v1 打包工作台。它把配置编辑、输入文件整理、签名选项、调用官方 packager 和实时日志放在同一个桌面界面里。

- Release：<https://github.com/Anyinstaller/anyinstaller-packager-gui/releases/tag/v0.2.0>
- 市场目录：<https://cdn.jsdelivr.net/gh/Anyinstaller/client-backend-marketplace@main/catalog.toml>

> 协议规范位于同工作区的 `aip/` 仓库。GUI 只编辑 AIP v1 已定义字段；最终校验、签名和自解包验证仍由 `packager.exe` 完成。

## 功能

- **AIP v1 配置编辑**：应用名称、图标、安装背景、默认安装位置、开始菜单文件夹和桌面快捷方式。
- **统一图标**：GUI、packager、client、backend 和 stub 使用同一套 `band.png`/`band.ico` 资源。
- **背景完整支持**：默认白色、图片、`#RRGGBB` 纯色，以及 `acrylic`、`acrylicBlur`、`mica`、`windowsManaged`、`transparent`。
- **配置往返**：载入和保存 `config.toml`；保存时保留原文件中的注释和未知键。
- **构建目录整理**：把 stub、client、backend、payload 自动同步为 packager 要求的固定文件名。
- **客户端资源合并**：选择 PyInstaller onedir 的 `assets` 目录，递归合并到 `build/assets`。
- **签名模式**：本地未签名、当前用户证书存储指纹、PFX 文件；未签名仅适合调试，不推荐正式发布。
- **组件市场**：直接集成在“运行组件”页，可从 jsDelivr 选择/下载客户端、后端和 stub，或导入本机组件，并强制校验 SHA-256。
- **独立设置页**：集中管理界面语言、GitHub 加速、出口 IP 自动检测、工作区组件自动定位和本机缓存路径。
- **国际化**：支持 English、简体中文、हिन्दी、Español、Français、العربية，切换后立即生效并持久化；阿拉伯语界面使用 RTL 布局。
- **GitHub 加速**：通过 `ipwho.is` 检测出口 IP；中国 CN 首次默认启用 `https://gh.meali.top/`，支持自定义、手动开关和直连回退。
- **实时构建日志**：后台线程执行 packager，不阻塞 UI；stdout/stderr 实时进入右侧日志面板。
- **工作区自动定位**：从当前目录或可执行文件位置向上寻找 `packager/`、`stub/`、`voidbackend/` 等同级仓库。
- **保留本机路径**：市场只负责下载和填充路径，stub、client、backend 始终可以在构建输入页手动覆盖。
- **无头验收**：支持 Rune 的 `--snap`、`--tree` 和场景参数，便于自动检查界面。

## 环境要求

- Windows 10/11 x64
- Rust 1.95+，MSVC toolchain
- 官方 `packager.exe`
- 发布包需要公网代码签名证书与 `signtool.exe`

首次构建 Rune 会下载并编译较多 GPU/UI 依赖，耗时明显长于普通 Rust 应用。

## 构建与运行

```powershell
cargo run
```

Release：

```powershell
cargo build --release
```

产物：

```text
target\release\anyinstaller-packager-gui.exe
```

如果 GUI 与 `packager.exe` 位于同一目录，GUI 会自动找到 packager。也可以在同工作区使用“重新自动定位”，或手动选择。

### 一键构建 GUI + packager 目录

```powershell
.\build.ps1
```

脚本会生成：

```text
dist\
├─ anyinstaller-packager-gui.exe
└─ packager.exe
```

这不是最终 AIP 安装器；它只是打包工作台的分发目录。

## 使用流程

1. **应用信息**：填写应用名称，选择图标和背景。
2. **安装默认值**：填写 `installLocation`，选择是否创建开始菜单文件夹和桌面快捷方式。
3. **运行组件**：在市场选择或下载组件、导入本机组件目录，并检查 `stub.exe`、`client.exe`、`backend.exe`、`payload.zip` 与客户端 `assets` 路径。
4. **签名与打包**：选择 packager 和输出位置，配置签名方式，点击“开始打包”。

网络、语言和自动检测配置位于左侧独立“设置”页。

点击打包时 GUI 会：

1. 做一轮快速预校验，并将 UI 定位到第一项错误。
2. 创建构建目录。
3. 把输入文件同步为 `stub.exe`、`client.exe`、`backend.exe`、`payload.zip`。
4. 递归合并客户端资源到 `assets/`。
5. 写入无 BOM 的 UTF-8 `config.toml`。
6. 创建输出父目录。
7. 在后台运行 `packager build ...`。
8. 实时显示 packager 输出，并用退出码更新构建状态。

同步采用合并写入，不会自动删除构建目录里已有的资源文件。这样可以避免 GUI 意外删除用户文件，但发布前仍应检查旧资源是否残留。

## 组件市场

市场索引通过 jsDelivr 获取：

```text
https://cdn.jsdelivr.net/gh/Anyinstaller/client-backend-marketplace@main/catalog.toml
```

目录使用 TOML，每个条目包含：

- 组件 id、名称、类型（client/backend/stub）和版本
- GitHub 仓库名、Release 标签和 ZIP 资产名
- 开源许可证和源码仓库
- ZIP 内可执行文件路径与可选 assets 目录
- SHA-256 和是否官方维护

Release 直链按以下形式构造：

```text
https://github.com/{repository}/releases/download/{release_tag}/{asset}
```

下载流程：

1. 优先使用已缓存且通过 SHA-256 的组件。
2. 可选使用 GitHub 加速前缀；加速失败或哈希不匹配时回退 GitHub 直连。
3. 下载 ZIP 后校验 SHA-256。
4. 拒绝加密条目、路径穿越、符号链接、超大条目和超大解压体积。
5. 安全解压到本机缓存，再自动填入构建输入路径。

缓存位置：

```text
%LOCALAPPDATA%\Anyinstaller\PackagerGUI\marketplace\
```

市场选择不会锁死输入；始终可以在“构建输入”页浏览并选择本机组件。

### GitHub 加速

GUI 在启动时自动通过 `https://ipwho.is/` 检测出口 IP，并自动定位同工作区组件。国内出口自动开启加速，海外出口自动关闭；如果 `ipwho.is` 无法访问，默认按国内处理并开启。左侧“设置”页会显示出口 IP，也可重新检测、修改加速前缀或手动开关。默认加速前缀：

```text
https://gh.meali.top/
```

手动设置保存在：

```text
%LOCALAPPDATA%\Anyinstaller\PackagerGUI\settings.toml
```

加速前缀只用于 GitHub Release 下载；市场目录始终通过 jsDelivr HTTPS 获取。关闭加速后直接使用 GitHub。无论加速开关如何，发行的 ZIP 都必须通过目录内 SHA-256。

## 签名

使用 SignPath Foundation 公益签名的 GitHub Actions 接入方式见 [`docs/signpath.md`](docs/signpath.md)。

### 本地未签名

```text
--unsigned
```

适合本地测试；正式发布**不推荐使用**。

### 证书存储

填写当前用户证书存储中的 40 位 SHA-1 指纹：

```text
--thumbprint <40-hex>
```

### PFX 文件

GUI 只填写 PFX 路径和密码环境变量名，不保存密码值。启动 GUI 前在当前终端设置环境变量：

```powershell
$env:AIP_PFX_PASSWORD = "your-pfx-password"
cargo run --release
```

使用发布签名时会执行：

```text
packager build --build-dir ... --output ... --pfx ... --pfx-password-env AIP_PFX_PASSWORD --timestamp-url ...
```

不要把密码写入脚本、仓库、配置文件或命令历史。

## 无头场景

```powershell
cargo run -- --snap target\ui-app.png --scene app
cargo run -- --snap target\ui-defaults.png --scene defaults
cargo run -- --snap target\ui-market.png --scene market
cargo run -- --snap target\ui-settings.png --scene settings
cargo run -- --snap target\ui-settings-en.png --scene english
cargo run -- --snap target\ui-settings-ar.png --scene arabic
cargo run -- --snap target\ui-inputs.png --scene inputs
cargo run -- --snap target\ui-package.png --scene package
cargo run -- --snap target\ui-pfx.png --scene pfx
```

元素树：

```powershell
cargo run -- --tree --scene inputs
```

## 代码结构

| 文件 | 职责 |
| --- | --- |
| `src/main.rs` | Rune `AppSpec`、场景和窗口配置 |
| `src/model.rs` | 步骤、背景、签名模式、字段和共享 UI 类型 |
| `src/app.rs` | 应用状态、文件选择器、配置载入/保存、后台任务协调 |
| `src/config_file.rs` | AIP v1 `config.toml` 解析、校验、渲染与注释保留 |
| `src/validation.rs` | GUI 侧快速校验 |
| `src/build_job.rs` | 构建目录整理、packager 子进程和日志流 |
| `src/market.rs` | 市场目录、ipwho.is 检测、GitHub 加速、下载、SHA-256 和 ZIP 安全解压 |
| `src/i18n.rs` | English、简体中文、印地语、西班牙语、法语、阿拉伯语界面翻译 |
| `src/settings.rs` | 本机语言与加速设置持久化 |
| `src/detect.rs` | 同工作区组件自动定位 |
| `src/ui.rs` | 全部 Rune 视图和可复用控件 |

## 安全约束

- AIP 配置必须是无 BOM 的 UTF-8 TOML，最大 64 KiB。
- `installLocation` 必须是本地绝对盘符路径，并拒绝受保护目录、相对组件、设备名和 ADS。
- 图片源路径拒绝 URL、UNC、设备路径和 reparse point。
- 构建输入的同步只接受普通文件；客户端资源目录拒绝 reparse point。
- 市场 ZIP 必须通过 SHA-256；解压拒绝加密、符号链接、路径穿越和超限内容。
- 输出必须位于构建目录之外，并且 packager 不得覆盖输入文件。
- PFX 不能放在构建目录内。
- 正式发布包建议使用可信证书签名并加 RFC 3161 时间戳。
- 最终 AIP 校验以 packager 为准，GUI 的预校验不是安全边界的替代品。

## 许可证

MPL-2.0。Rune 使用 MIT 许可证。
