# 真实 API 接入方案

## 当前状态

已经接入的真实链路：

- Svelte 通过统一 `BackendTransport` 调用后端。
- 桌面端使用 Tauri `invoke`。
- Web 端使用 `danxi-server` 的 HTTP API。
- 社区登录/登出/会话状态：`POST /v1/session/login`、`POST /v1/session/logout`、`GET /v1/session/status`（桌面端对应 `community_login`、`community_logout`、`session_status` 命令）。
- 茶楼：支持分区与标签读取、帖子列表和详情、发帖、回复、点赞/点踩、收藏与举报；所有请求均在 Rust 层附加 Bearer Token，并在 401 时自动用 Refresh Token 换新后重试一次。
- 评教：随机评价、课程搜索与课程评价详情均通过 Rust 侧访问旦课 API，共享旦挞 Token，并支持自动 WebVPN。
- 校园服务：图书馆人数公开读取；食堂拥挤度、校车时刻和空教室通过 Rust 侧复用复旦 UIS 会话，空教室在校外自动回退 WebVPN。
- 复旦校园登录（id.fudan.edu.cn，即原 UIS 的继任系统）：`POST /v1/session/campus/login`、`POST /v1/session/campus/logout`。完整复刻 Flutter 客户端 V2 密码流程：authenticate 重定向取 `lck`/`entityId` → `queryAuthMethods` → `getJsPublicKey` → RSA 加密密码 `authExecute`。需要二次验证时返回结构化 `requiresSecondFactor` 状态；桌面端随后调用 `complete_campus_second_factor` 打开内嵌认证窗口，完成后导入教务与统一认证 Cookie、验证目标会话并保存凭证。浏览器版受跨域 HttpOnly Cookie 隔离限制，当前会明确提示改用桌面客户端。
- 桌面端社区 Token 与校园凭证使用 AES-256-GCM 加密文件保存，并自动迁移旧版明文 `secrets.json`；未重新使用系统钥匙串。Web 端凭证仅存在服务端会话内存中。
- 应用启动时先从前端公开状态快照恢复登录 UI，再调用本地状态接口并在后台校验真实会话，网络抖动不会阻塞首屏登录态。

首页日程与完整日程页均使用真实课表接口；茶楼动态也使用真实接口数据。

## 现有服务

| 领域 | 上游地址 | 认证方式 | 说明 |
| --- | --- | --- | --- |
| 茶楼 | `https://forum.fduhole.com/api` | Bearer JWT | 帖子、楼层、分区、消息 |
| 旦挞认证 | `https://auth.fduhole.com/api` | 邮箱密码 + Access/Refresh Token | 登录、刷新、用户信息 |
| 旦课 | `https://danke.fduhole.com/api` | 与树洞共享 JWT | 课程组、课程和评价 |
| 本科教务 | `https://fdjwgl.fudan.edu.cn` | 复旦统一认证 Cookie | 学期与课表 |
| 校园生活 | 图书馆、`my.fudan.edu.cn`、校车与教室状态服务 | 公开或复旦统一认证 Cookie | 人数、拥挤度、班次、空教室 |
| 统一身份认证 | `https://id.fudan.edu.cn` | 学号密码、可能需要二次验证 | 只由 Rust 层访问 |

## 前端调用方式

组件不得判断自己运行在桌面还是 Web，也不得直接拼接上游 URL：

```ts
const timetable = await backend.loadTimetable();
```

`BackendTransport` 的两个实现使用相同 DTO：

```text
TauriTransport → invoke("load_timetable")
WebTransport   → GET /v1/campus/timetable
```

## 桌面端认证

1. Svelte 调用 `campusLogin(username, password)`；后端自动判断本科或研究生课表系统。
2. Tauri command 将数据直接交给 Rust，不写入前端状态或日志。
3. Rust 使用独立 Cookie Jar 处理复旦登录重定向和二次验证。
4. 登录成功后仅把必要凭证写入应用数据目录中的加密凭证文件。
5. 页面只收到 `SessionStatus`，不收到原始 Cookie 或密码。

## Web 端认证

浏览器不能直接复用桌面实现，否则会遇到跨域、Cookie 域和凭证泄漏问题。Web 端流程为：

1. 浏览器通过 HTTPS 把登录请求发送到 `danxi-server`。
2. 网关创建服务端会话，在服务端保存复旦 Cookie 和社区 JWT。
3. 浏览器只保存 `HttpOnly + Secure + SameSite` 的不透明 Session Cookie。
4. 后续 `/v1/timetable`、`/v1/forum/holes` 请求由网关携带服务端凭证访问上游。
5. 部署时必须增加 CSRF 防护、登录限流、会话过期和审计日志脱敏。

## 实现顺序

1. ~~`SessionStore` 接口：桌面文件存储实现、Web 服务端会话实现。~~ 已完成（`danxi-core::session`、`src-tauri` FileSecretStore、`danxi-server` SessionRegistry）。
2. ~~社区登录：`POST /api/login`，并实现 `POST /api/refresh` 自动刷新。~~ 已完成（`danxi-core::forum::SessionManager`）。
3. ~~树洞列表：`GET /api/holes`，仅在 Rust 层附加 Bearer Token。~~ 已完成。
4. ~~旦课搜索：`GET /api/v3/course_groups/search`，复用社区 Token。~~ 已完成，并已接入课程详情与随机评价。
5. ~~复旦认证登录：独立 Cookie Jar，并自动识别本科生或研究生课表系统。~~ 密码登录与桌面端二次验证续接已完成（`danxi-core::campus`，新版 id.fudan.edu.cn 流程）。课表、图书馆人数、食堂拥挤度、校车时刻与空教室均已接入；Web 网关二次验证仍需可回调的服务端认证方案。

## 本地代理

Rust 后端支持统一上游代理：

```bash
DANXI_HTTP_PROXY=http://127.0.0.1:7897 pnpm dev:web-server
```

代理只在 Rust 后端生效，不把代理地址或上游凭证暴露给浏览器。
