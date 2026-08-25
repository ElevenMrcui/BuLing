.PHONY: help install gateway typecheck build lint test clean

help:
	@echo "不令 OPC · 常用命令"
	@echo ""
	@echo "  make install    安装依赖 (pnpm install)"
	@echo "  make gateway    启动本地网关 (http://127.0.0.1:17817) —— 扫描本机 AI CLI"
	@echo "  make typecheck  全仓 TS 类型检查"
	@echo "  make lint       全仓 Lint"
	@echo "  make test       全仓单测"
	@echo "  make build      全仓构建"
	@echo "  make clean      清理 node_modules / dist"
	@echo ""
	@echo "旧「企业级协作平台」骨架（NestJS / Next.js / Postgres）已归档到 legacy/，"
	@echo "不再对应任何 make 目标；如需参考，见 legacy/docs/*。"

install:
	pnpm install

gateway:
	pnpm --filter @buling/local-gateway start

build:
	pnpm build

lint:
	pnpm lint

typecheck:
	pnpm typecheck

test:
	pnpm test

clean:
	rm -rf node_modules apps/*/node_modules packages/*/node_modules apps/*/dist packages/*/dist
