# DanXi Next

面向复旦校园场景的新一代旦夕客户端。项目使用 Tauri 2、Svelte 5、TypeScript、UnoCSS 与 shadcn-svelte 组件模式，并从第一天起支持桌面与 Web 双运行时。

## 技术栈

- Tauri 2：桌面窗口、系统能力和前后端通信
- Svelte 5 + SvelteKit：共享界面与交互
- UnoCSS：原子样式和设计令牌
- shadcn-svelte：可复制、可维护的 UI 组件结构
- `danxi-core`：桌面端与 Web 网关共享的 Rust 业务核心
- `danxi-server`：Web 部署使用的 API 网关

## 本地运行

桌面端：

```bash
pnpm install
pnpm tauri dev
```

Web 端：

```bash
pnpm dev:web-server
pnpm dev:web
```

## 验证

```bash
pnpm check
pnpm build
cargo check --workspace
```

## 发布

GitHub Actions 会在推送 `v*` 标签时创建 Release，并分别构建以下六个平台产物：

- macOS ARM64 与 x64（DMG）
- Windows ARM64 与 x64（NSIS）
- Linux ARM64 与 x64（AppImage、DEB）

发布前必须同步更新 `package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 中的版本号。标签必须与版本完全一致，例如：

```bash
pnpm release:check -- v0.1.0
git tag -a v0.1.0 -m "DanXi Next v0.1.0"
git push origin v0.1.0
```

工作流会先创建草稿 Release；仅当六个平台全部构建成功后才会自动发布。当前产物未配置 Apple 或 Windows 代码签名。

## 架构

```text
Svelte UI → runtime transport ┬→ Tauri invoke ─┐
                              └→ Web HTTP API ─┤
                                               ↓
                                          danxi-core
```

前端不直接持有密码、Cookie 或长期 Token。真实接口方案见 `docs/api-integration.md`，迁移计划见 `docs/migration-plan.md`，Web 部署说明见 `docs/web-deployment.md`。
