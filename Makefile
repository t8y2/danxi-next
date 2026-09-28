# DanXi Next 开发入口
#
# 常用命令：
#   make dev          启动桌面端（Tauri dev）
#   make dev-web      同时启动 Web 网关和 Web 前端（浏览器访问 http://127.0.0.1:1420）
#   make check        前端 + Rust 全量检查
#   make build        构建前端产物
#   make clean        清理构建产物

PNPM := pnpm
CARGO := cargo

.DEFAULT_GOAL := help

.PHONY: help
help: ## 显示帮助
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

.PHONY: dev
dev: ## 启动桌面端（首次编译 Rust 需几分钟）
	@# 注意：tauri dev 只监听 src-tauri/，改动 crates/ 后需 touch src-tauri 触发重编译，
	@# 或直接重启本命令。
	$(PNPM) tauri dev

.PHONY: dev-web
dev-web: ## 同时启动 Web 网关（:8787）与 Web 前端（:1420）
	@echo "启动 Web 网关与前端，浏览器访问 http://127.0.0.1:1420"
	@$(MAKE) -j2 dev-web-server dev-web-front

.PHONY: dev-web-server
dev-web-server: ## 仅启动 Web API 网关（127.0.0.1:8787）
	$(CARGO) run -p danxi-server

.PHONY: dev-web-front
dev-web-front: ## 仅启动 Web 前端（127.0.0.1:1420，代理 /api 到网关）
	$(PNPM) dev:web

.PHONY: check
check: check-front check-rust ## 全量检查（前端 + Rust）

.PHONY: check-front
check-front: ## 前端类型检查
	$(PNPM) check

.PHONY: check-rust
check-rust: ## Rust 工作区检查
	$(CARGO) check --workspace

.PHONY: test
test: ## 运行 Rust 单元测试
	$(CARGO) test --workspace

.PHONY: build
build: ## 构建前端产物
	$(PNPM) build

.PHONY: icons
icons: ## 从仓库内的 1024px 源图重新生成全套图标
	$(PNPM) tauri icon src-tauri/icon-source.png

.PHONY: clean
clean: ## 清理构建产物
	$(CARGO) clean
	rm -rf build .svelte-kit node_modules/.vite
