# Web 部署设计

## 总体结构

```text
浏览器中的 Svelte SPA
        ↓ HTTPS JSON
danxi-server Web API 网关
        ↓
danxi-core 共享业务逻辑
        ↓
树洞服务 / 旦课服务 / 复旦校方系统
```

桌面端通过 Tauri invoke 直接调用同一份 `danxi-core`。前端页面只依赖统一的 TypeScript API 接口，因此无需为 Web 和桌面维护两套组件。

## 为什么需要 Web API 网关

- 复旦校方登录涉及跨站 Cookie、重定向和可能的二次验证，不能依赖浏览器直接跨域调用。
- 密码、长期 Token 和校方 Cookie 不应进入 `localStorage`、页面状态或前端日志。
- 网关可以统一处理会话、频率限制、日志脱敏和上游变更。

## 开发命令

```bash
# 终端一：启动 Web API 网关
pnpm dev:web-server

# 终端二：启动浏览器前端
pnpm dev:web
```

生产构建：

```bash
VITE_DANXI_API_BASE_URL=https://your-api.example.com pnpm build:web
```

生产环境应通过 `DANXI_ALLOWED_ORIGIN` 限制网关允许的前端来源，并在反向代理层启用 HTTPS、安全响应头和请求频率限制。
