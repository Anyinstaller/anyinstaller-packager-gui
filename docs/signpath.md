# SignPath 公益签名集成

SignPath Foundation 为符合条件的开源项目提供公益代码签名服务。私钥由 SignPath 托管，CI 只提交待签名制品，不接触证书私钥。

- SignPath Foundation：<https://signpath.org/>
- 提交签名请求的 GitHub Action：<https://github.com/signpath/github-action-submit-signing-request>

> 需要先向 SignPath Foundation 申请项目并获批准。项目、签名策略和制品配置由 SignPath 管理员配置。

## 适用对象

- anyinstaller-packager-gui.exe
- packager.exe
- voidclient 中的 client.exe
- backend.exe
- 最终打包生成的 AIP 安装器 foo.exe

最终安装器建议在 AIP 归档完成后，对最终 foo.exe 签名并加 RFC 3161 时间戳。不要只签内嵌的 client.exe 或 backend.exe 来代替最终安装器签名。

## 1. 准备 GitHub Secrets

在 GitHub 仓库或组织 Secrets 中配置：

| Secret | 说明 |
| --- | --- |
| SIGNPATH_API_TOKEN | SignPath API Token |
| SIGNPATH_ORGANIZATION_ID | SignPath 组织 ID |
| SIGNPATH_PROJECT_SLUG | SignPath 项目标识 |
| SIGNPATH_POLICY_SLUG | 签名策略，例如 release-signing |
| SIGNPATH_ARTIFACT_CONFIGURATION_SLUG | 制品配置标识，例如 windows-exe |

不要把证书、PFX 密码或 SignPath Token 写入仓库。

## 2. GitHub Actions 示例

下面的工作流先构建并上传待签名 ZIP，再提交 SignPath，最后上传签名结果：

```yaml
name: Build and Sign

on:
  workflow_dispatch:
  push:
    tags:
      - "v*"

permissions:
  contents: read

jobs:
  build-and-sign:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Checkout packager
        uses: actions/checkout@v4
        with:
          repository: Anyinstaller/packager
          path: packager

      - name: Build
        shell: pwsh
        run: |
          cargo build --release
          cargo build --release --manifest-path packager/Cargo.toml
          New-Item -ItemType Directory -Force target\unsigned | Out-Null
          Copy-Item target\release\anyinstaller-packager-gui.exe target\unsigned\
          Copy-Item packager/target/release/packager.exe target\unsigned\
          Compress-Archive target\unsigned\* target\unsigned\windows-x64.zip -Force

      - name: Upload unsigned artifact
        id: upload
        uses: actions/upload-artifact@v4
        with:
          name: unsigned-windows-x64
          path: target/unsigned/windows-x64.zip

      - name: Submit SignPath signing request
        uses: signpath/github-action-submit-signing-request@v1
        with:
          api-token: ${{ secrets.SIGNPATH_API_TOKEN }}
          organization-id: ${{ secrets.SIGNPATH_ORGANIZATION_ID }}
          project-slug: ${{ secrets.SIGNPATH_PROJECT_SLUG }}
          signing-policy-slug: ${{ secrets.SIGNPATH_POLICY_SLUG }}
          artifact-configuration-slug: ${{ secrets.SIGNPATH_ARTIFACT_CONFIGURATION_SLUG }}
          github-artifact-id: ${{ steps.upload.outputs.artifact-id }}
          wait-for-completion: true
          output-artifact-directory: target/signed

      - name: Upload signed package
        uses: actions/upload-artifact@v4
        with:
          name: signed-windows-x64
          path: target/signed/**
```

SignPath 的制品配置需要指定 ZIP 内哪些 PE 文件需要 Authenticode 签名。通常选择：

- anyinstaller-packager-gui.exe
- packager.exe

如果 Release 还包含 client.exe、backend.exe 或 stub.exe，也应在 SignPath 制品配置中逐个声明。

## 3. 最终 AIP 安装器签名

AIP 要求最终单文件安装器被 Authenticode 签名覆盖。推荐流程：

1. 构建 stub.exe、client.exe、backend.exe 和 payload.zip。
2. 使用 packager 生成 foo.exe 并完成自解包校验。
3. 对这个最终 foo.exe 再提交 SignPath 或本地 signtool 签名。
4. 验证签名、时间戳和 AIP 清单仍然有效。

本地签名仍可使用 packager 现有参数：

```text
packager build --build-dir build --output dist/foo.exe --thumbprint <SHA1>
```

或使用 SignPath 签名后的 packager.exe 执行同样的流程。

## 4. 发布检查

- 签名证书链受信任
- 使用 RFC 3161 时间戳
- 签名覆盖最终安装器
- 签名后再次运行签名验证与 AIP 清单校验
- Release 上传签名后的最终文件，不混用未签名构建

代码签名用于证明发布者身份和文件完整性；市场下载仍应继续校验目录中的 SHA-256。
