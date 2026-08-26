.PHONY: help install gateway desktop-dev desktop-build desktop-check runtime-test runtime-check typecheck build lint test clean

help:
	@echo "不令 OPC · 常用命令"
	@echo ""
	@echo "  make install         安装 JS 依赖 (pnpm install)"
	@echo ""
	@echo "  == 桌面 App (Tauri 2) =="
	@echo "  make desktop-dev     启动 Tauri dev（会自动起 Vite + 打开窗口，需要图形环境）"
	@echo "  make desktop-build   打包桌面 App（生产版）"
	@echo "  make desktop-check   仅编译前端 + cargo check（无窗口，无图形环境也能跑）"
	@echo ""
	@echo "  == Runtime (Rust · SQLite storage) =="
	@echo "  make runtime-check   cargo check 全部 runtime crate"
	@echo "  make runtime-test    cargo test 全部 runtime crate（含 sqlite / FTS5 集成测试）"
	@echo ""
	@echo "  == 本地网关 (Node · daemon) =="
	@echo "  make gateway         启动本地网关 http://127.0.0.1:17817"
	@echo ""
	@echo "  == 全仓 =="
	@echo "  make typecheck       全仓 TS 类型检查"
	@echo "  make lint            全仓 Lint"
	@echo "  make test            全仓单测"
	@echo "  make build           全仓构建"
	@echo "  make clean           清理 node_modules / dist / target"
	@echo ""
	@echo "旧「企业级协作平台」骨架 (NestJS/Next.js/Postgres) 已归档 legacy/。"

install:
	pnpm install

# --- 桌面 App ---

desktop-dev:
	cd apps/desktop/src-tauri && cargo run

desktop-build:
	pnpm --filter @buling/desktop build
	cd apps/desktop/src-tauri && cargo build --release

desktop-check:
	pnpm --filter @buling/desktop typecheck
	pnpm --filter @buling/desktop build
	cd apps/desktop/src-tauri && cargo check

# --- Runtime ---

runtime-check:
	cd runtime && cargo check

runtime-test:
	cd runtime && cargo test

# --- 本地网关 ---

gateway:
	pnpm --filter @buling/local-gateway start

# --- 全仓 ---

build:
	pnpm build

lint:
	pnpm lint

typecheck:
	pnpm typecheck

test:
	pnpm test
	cd runtime && cargo test

clean:
	rm -rf node_modules apps/*/node_modules packages/*/node_modules apps/*/dist packages/*/dist
	rm -rf runtime/target apps/desktop/src-tauri/target
