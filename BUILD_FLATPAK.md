# Szyszka Flatpak 构建指南

> 本文档供 AI 编程助手在协助构建和发布 Flatpak 版本时参考。
> **AI 应自动化完成全部流程**：版本号更新、commit、tag、push、flatpak 构建、bundle 导出、GitHub Release 创建，无需用户手动执行任何步骤。

## 一、前置条件

确保系统已安装：

```bash
flatpak --version       # 需要 >= 1.12
flatpak-builder --version
cargo --version
```

需要添加 Flathub 运行时源（首次）：

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.gnome.Platform//50 org.gnome.Sdk//50
flatpak install --user flathub org.freedesktop.Sdk.Extension.rust-stable//25.08
```

## 二、版本发布完整流程

> **AI 执行说明**：以下所有步骤应由 AI 自动完成，无需用户手动操作。AI 应按顺序执行：查看 git 历史 → 更新版本号 → 更新 metainfo → vendor → commit → tag → push → flatpak 构建 → bundle 导出 → 安装验证 → GitHub Release 创建。

### 2.1 版本发布提交规范

**每次版本发布包含多个功能提交 + 1 个版本更新提交**：

1. **功能提交**：将代码变更提交到主分支（数量不限）
2. **发布提交**：修改版本号到 `Cargo.toml` 和 `metainfo.xml`，并重新生成 `Cargo.lock`

**版本发布 commit 格式**：

```bash
git commit -m "release: v4.1.0"
```

> `metainfo.xml` 中的 `version` 属性为纯版本号（如 `4.1.0`），不含 `v` 前缀。

### 2.2 确定更新内容范围

在填写版本更新日志时，需要通过 Git 提交历史来确定新版本包含的变更：

```bash
# 查看当前版本与上一版本之间的所有提交
git log v4.0.0..HEAD --oneline

# 查看详细变更内容（用于总结更新日志）
git log v4.0.0..HEAD --stat --name-only
```

> 💡 **提示**：当 Git 提交记录中没有明确的版本标记时，请查看最近 20-50 条提交历史，结合 README 和代码变更来判断版本边界。通常版本号更新提交会包含 `Cargo.toml` 和 `metainfo.xml` 的修改。

---

### 2.3 更新版本号

需要修改 **3 个文件**：

| 文件 | 修改内容 |
| --- | --- |
| `Cargo.toml` | 第 5 行 `version = "x.y.z"`（此为唯一版本源） |
| `Cargo.lock` | 运行 `cargo check` 自动更新 |
| `data/com.github.samfic.szyszka.metainfo.xml` | 在 `<releases>` 内新增 `<release>` 条目，按版本号**从新到旧**排列，`date` 使用当天日期（格式 `YYYY-MM-DD`） |

`metainfo.xml` 新增条目的格式示例：

```xml
<release version="4.1.0" date="2026-07-10">
  <description>
    <p>新特性与修复：</p>
    <ul>
      <li>简单描述：具体变更 1</li>
      <li>简单描述：具体变更 2</li>
    </ul>
  </description>
</release>
```

> 💡 `date` 属性使用当天日期，格式为 `YYYY-MM-DD`。

### 2.4 提交、打标签与推送

AI 应直接执行以下命令，无需询问用户：

```bash
# 更新 Cargo.lock
cargo check

git add Cargo.toml Cargo.lock data/com.github.samfic.szyszka.metainfo.xml
git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
git push && git push origin vX.Y.Z
```

### 2.5 构建 Flatpak 包

#### 方式 A：仅本地安装（快速验证）

```bash
# 1. 生成 vendored 依赖
cargo vendor

# 2. 构建并安装
flatpak-builder build-dir flatpak/com.github.samfic.szyszka.yml --user --install --force-clean

# 3. 清理
rm -rf vendor/
```

- `build-dir/` 是临时构建目录（已在 `.gitignore` 中忽略）
- `--force-clean` 会删除旧的构建目录，避免缓存冲突
- `--user --install` 构建完成后自动安装到当前用户环境

#### 方式 B：构建可分发的 .flatpak 文件（用于发布）

> ⚠️ **重要说明**：`flatpak-builder --repo=flatpak-repo build-dir ...` 会创建两个独立的目录：
>
> - `build-dir/` — **构建目录**，存放编译产物（可被 `--force-clean` 清理，已在 `.gitignore` 中忽略）
> - `flatpak-repo/` — **仓库目录**，由 `--repo` 指定的地方，`build-bundle` 必须从此读取仓库数据
>
> 如果将 `build-dir` 误用作 `build-bundle` 的参数会报错。

```bash
# 步骤 1：生成 vendored 依赖
cargo vendor

# 步骤 2：构建到本地仓库
flatpak-builder --repo=flatpak-repo build-dir flatpak/com.github.samfic.szyszka.yml --force-clean

# 步骤 3：从本地仓库导出单文件 bundle
flatpak build-bundle flatpak-repo szyszka-X.Y.Z.flatpak com.github.samfic.szyszka

# 步骤 4：清理
rm -rf vendor/
```

> ⚠️ **常见错误**：`build-bundle` 的第一个参数必须是 **本地仓库目录**（即 `--repo=` 指定的目录），**不是**构建目录 `build-dir/`。如果传入 `build-dir/` 会报错：
>
> ```
> error: 'build-dir' is not a valid repository: opening repo: opendir(objects): No such file or directory
> ```

### 2.6 验证构建结果

```bash
# 检查 bundle 文件大小
ls -lh szyszka-*.flatpak

# 确认 flathub 源已添加（首次需要）
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# 安装并运行验证
flatpak install --user --or-update -y szyszka-X.Y.Z.flatpak
flatpak run com.github.samfic.szyszka

# 确认 metainfo 中的版本号正确
flatpak info com.github.samfic.szyszka
```

> 💡 **交互式确认**：`flatpak install` 可能弹出确认提示 `[Y/n]`，在脚本中可添加 `--noninteractive` 标志：
>
> ```bash
> flatpak install --noninteractive --user --or-update szyszka-X.Y.Z.flatpak
> ```

### 2.7 修改源码后重新构建

如果在构建 flatpak **之后**又修改了源码（如修正版本号、更新翻译等），**必须重新执行完整构建流程**（`cargo vendor` → `flatpak-builder --repo=...` + `build-bundle`），否则安装的仍是旧构建。仅重新 `cargo build` 不会影响已安装的 flatpak 包。

## 三、项目结构说明

```
flatpak/
  com.github.samfic.szyszka.yml   ← Flatpak 构建清单（manifest）
Cargo.toml                        ← Cargo 构建配置（**唯一版本源**）
Cargo.lock                        ← 依赖锁定文件（cargo check 自动更新）
data/
  com.github.samfic.szyszka.metainfo.xml  ← AppStream 元数据（版本记录在此）
  com.github.samfic.szyszka.desktop       ← 桌面入口文件
  icons/com.github.samfic.szyszka.svg     ← 应用图标
```

### 构建清单结构

`flatpak/com.github.samfic.szyszka.yml` 包含一个模块：

1. **`szyszka`**：主应用，使用 `type: dir` 指向本地源码 + vendored Cargo 依赖

构建流程：
- `cargo vendor` 将所有 crate 下载到 `vendor/` 目录
- `.cargo/config.toml`（inline source）告诉 cargo 使用 vendored 依赖
- `cargo --offline build --release` 离线编译

> 说明：`type: dir` 在 CI 中同样可用——`release.yml` 的 Flatpak job 在容器内 checkout 仓库后，清单里的 `path: ..` 正好指向仓库根目录，因此无需改成 `type: archive` 或 `type: git`。

## 四、构建配置的关键细节

### 4.1 运行时版本

当前使用 `org.gnome.Platform` **runtime-version: 50**（对应 GNOME 50）。

如果需要升级运行时版本（例如 GNOME 51 发布后），需要同时更新：

- `flatpak/com.github.samfic.szyszka.yml` 中的 `runtime-version`
- CI/CD 中安装的运行时版本

### 4.2 权限（finish-args）

```yaml
- --share=ipc          # 进程间通信（X11 需要）
- --socket=fallback-x11 # X11 回退
- --socket=wayland     # Wayland 显示协议
- --device=dri         # GPU 硬件加速
```

> 注意：Szyszka 不需要 `--filesystem=host`，因为用户通过 GTK 文件选择器添加文件，不需要直接访问文件系统。

### 4.3 Rust SDK 扩展

```yaml
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
```

Rust 工具链通过 SDK 扩展提供，无需在 manifest 中单独构建。

## 五、常见问题排查

### 5.1 构建失败：缺少 vendored 依赖

```
error: no matching package named `xxx` found
note: offline mode (via `--offline`) can sometimes cause surprising resolution failures
```

**解决**：确保在构建前执行了 `cargo vendor`，并且 `.cargo/config.toml` 正确指向 `vendor/` 目录。

### 5.2 构建失败：Rust 扩展未安装

```
error: /usr/lib/sdk/rust-stable/bin/cargo: No such file or directory
```

**解决**：

```bash
flatpak install --user flathub org.freedesktop.Sdk.Extension.rust-stable//25.08
```

### 5.3 构建失败：GNOME SDK 版本不匹配

**解决**：检查 `flatpak/com.github.samfic.szyszka.yml` 中的 `runtime-version` 是否与已安装的 SDK 版本一致。

### 5.4 本地安装时交互式提示

```
Configure this as new remote 'flathub' [Y/n]:
```

如果在自动化脚本中运行，可以预先添加 flathub 远程源避免提示：

```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

### 5.5 网络超时/重试

构建时下载 vendored crate 可能因网络不稳定失败：

```bash
# 失败后直接重试即可，vendor 目录会被缓存
cargo vendor
flatpak-builder --repo=flatpak-repo build-dir flatpak/com.github.samfic.szyszka.yml --force-clean
```

### 5.6 文件路径不被 Git 跟踪

构建产生的 `*.flatpak`、`build-dir/`、`flatpak-repo/`、`.flatpak-builder/`、`vendor/` 等均已包含在 `.gitignore` 中。如果希望保留某个版本的 bundle，手动复制到其他目录或发布到 GitHub Releases。

### 5.7 构建产物清理

构建产物不会自动清理，建议定期删除旧版本 bundle 以节省磁盘空间：

```bash
# 删除旧版本 bundle，保留当前版本
find . -name "szyszka-*.flatpak" ! -name "szyszka-4.1.0.flatpak" -delete

# 清理旧的构建目录
rm -rf build-dir/ flatpak-repo/ .flatpak-builder/ vendor/
```

### 5.8 跨版本构建注意事项

从旧版本构建升级时，注意 `.flatpak-builder/` 缓存可能导致问题。使用 `--force-clean` 可确保完全重新构建。

## 六、快速参考命令

> 以下所有步骤由 AI 自动执行，用户无需手动操作。

```bash
# ──────────────────────────────────────
# 完整的版本发布流程（AI 全自动执行）
# ──────────────────────────────────────
# 1. git log 查看上一版本到现在的提交，总结更新内容
# 2. 编辑 Cargo.toml 更新版本号
# 3. cargo check 更新 Cargo.lock
# 4. 编辑 metainfo.xml 添加发布记录
# 5. git add Cargo.toml Cargo.lock data/com.github.samfic.szyszka.metainfo.xml && git commit -m "release: vX.Y.Z" && git tag vX.Y.Z && git push && git push origin vX.Y.Z
# 6. cargo vendor
# 7. flatpak-builder --repo=flatpak-repo build-dir flatpak/com.github.samfic.szyszka.yml --force-clean
# 8. flatpak build-bundle flatpak-repo szyszka-X.Y.Z.flatpak com.github.samfic.szyszka
# 9. flatpak install --user --or-update -y szyszka-X.Y.Z.flatpak
# 10. rm -rf vendor/
# 11. gh release create vX.Y.Z --title "Szyszka vX.Y.Z" --notes "..." szyszka-X.Y.Z.flatpak

# ──────────────────────────────────────
# 仅本地安装验证（快速开发测试）
# ──────────────────────────────────────
cargo vendor
flatpak-builder build-dir flatpak/com.github.samfic.szyszka.yml --user --install --force-clean
rm -rf vendor/
```

---

## 七、发布到 GitHub Releases

AI 应使用 `gh` CLI 自动完成，无需用户手动操作。

### 7.1 前置检查

```bash
# 确认 gh 已安装
gh --version

# 确认已登录（未登录则提示）
gh auth status 2>&1 || {
  echo "未登录 GitHub CLI，请先执行 gh auth login"
  exit 1
}
```

### 7.2 创建 Release

创建 Release 并上传 Flatpak bundle（注意格式按照模板写，中英文日志都有冒号）：

```bash
gh release create vX.Y.Z --title "Szyszka vX.Y.Z" --notes "$(cat <<'EOF'
### 主要改进

- **简洁描述**：详细内容
- **简洁描述**：详细内容

### Improvements

- **Brief description**: Detailed content
- **Brief description**: Detailed content

EOF
)" szyszka-X.Y.Z.flatpak
```

> 💡 Release notes 内容应从 `metainfo.xml` 的 `<release>` 条目中提取，保持一致。

---

## 八、GitHub Actions 自动化

项目配置了 `.github/workflows/release.yml`，当推送 `v*` 标签时自动完成跨平台构建与发布：

1. 并行构建 3 个产物：Flatpak bundle（Linux 平台专用）、Windows 二进制、macOS 二进制
2. 从 `metainfo.xml` 提取对应版本的更新说明
3. 创建（唯一的）GitHub Release，将全部产物一并上传

### 8.1 触发方式

```bash
# 手动触发完整发布流程
git tag v4.1.0
git push origin v4.1.0
# → 自动触发 CI 构建 + 发布
```

### 8.2 CI 构建流程

发布流程中仅 Flatpak job 使用仓库内的 manifest（`flatpak/com.github.samfic.szyszka.yml`，其中 `type: dir`、`path: ..` 指向仓库根目录），无需 `type: archive` 或 tarball；三平台二进制 job 则直接 `cargo build --release`，因此：

1. 推送 `v*` 标签后，CI 自动 checkout 仓库（清单的 `path: ..` 即指向仓库根）
2. Flatpak job 自动运行 `cargo vendor` 生成 vendored 依赖，并用 `type: dir` 源码构建 bundle
3. Windows / macOS job 各自安装对应 GTK 依赖（MSYS2 / `brew`）后 `cargo build --release`；Linux 平台不产出独立原生二进制，仅由 Flatpak job 提供安装包
4. 统一的 `release` job 下载全部 artifact，从 `metainfo.xml` 提取 notes，一次性创建 GitHub Release 并上传所有产物

### 8.3 本地模拟 CI 构建

CI 使用的是 `type: dir` 清单（与前文本地构建完全一致），因此本地模拟 CI 构建最简单的方式就是**直接用同一份 manifest**，无需改成 archive：

```bash
# 1. 生成 vendored 依赖
cargo vendor

# 2. 构建（与 release.yml 中的命令一致）
flatpak-builder --repo=flatpak-repo build-dir flatpak/com.github.samfic.szyszka.yml --force-clean

# 3. 导出 bundle
flatpak build-bundle flatpak-repo szyszka-X.Y.Z.flatpak com.github.samfic.szyszka

# 4. 清理
rm -rf vendor/
```

> 说明：CI 容器内 checkout 后清单的 `path: ..` 指向仓库根，与本地执行时目录结构相同，所以本地命令和 CI 命令一致。
