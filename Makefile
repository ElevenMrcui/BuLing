.PHONY: help install db db-down db-logs migrate generate dev api web gateway build lint typecheck test clean

help:
	@echo "常用命令："
	@echo "  make install    安装依赖 (pnpm install)"
	@echo "  make db         启动本地依赖（Postgres + Redis）"
	@echo "  make db-down    停止本地依赖"
	@echo "  make migrate    执行 Prisma 迁移"
	@echo "  make generate   生成 Prisma Client"
	@echo "  make dev        并行启动 API 与 Web（依赖需先 make db）"
	@echo "  make api        仅启动 API (http://localhost:13500)"
	@echo "  make web        仅启动 Web (http://localhost:13000)"
	@echo "  make gateway    启动本地网关 (http://127.0.0.1:17817) —— 用于扫描本机 AI CLI"
	@echo "  make typecheck  全仓 TS 类型检查"
	@echo "  make lint       全仓 Lint"
	@echo "  make test       全仓单测"
	@echo "  make build      全仓构建"

install:
	pnpm install

db:
	docker compose -f infra/docker-compose.yml up -d
	@echo "等待数据库健康..."
	@until docker compose -f infra/docker-compose.yml ps postgres | grep -q healthy; do sleep 1; done
	@echo "OK"

db-down:
	docker compose -f infra/docker-compose.yml down

db-logs:
	docker compose -f infra/docker-compose.yml logs -f

migrate:
	pnpm --filter @buling/api prisma:migrate

generate:
	pnpm --filter @buling/api prisma:generate

dev:
	pnpm dev

api:
	pnpm --filter @buling/api dev

web:
	pnpm --filter @buling/web dev

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
	rm -rf node_modules apps/*/node_modules packages/*/node_modules apps/*/dist packages/*/dist apps/*/.next
