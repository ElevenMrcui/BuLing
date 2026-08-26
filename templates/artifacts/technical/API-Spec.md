---
kind: api-spec
version: 1
producer: 架构师
inputs:
  - prd
  - architecture
  - db-design
project: <auto-fill>
created_at: <auto-fill-iso>
---

# API 规范 · API Spec

<!--
架构师产出。硬约束：
1. 每个 endpoint 必须给出：方法 · 路径 · 请求体 · 响应体 · 错误码 · 鉴权
2. 请求体 / 响应体用完整 JSON schema 或 TypeScript interface 表达，禁止"..."
3. 前端与后端都从这份 spec 起步；spec 变更 = 通知双方
-->

## 1. 总览

- **协议**：HTTP/1.1 over TLS
- **格式**：application/json；请求头 `Content-Type: application/json`
- **鉴权**：[Bearer JWT / API Key / Session Cookie]，见 Security.md
- **版本前缀**：`/api/v1/`
- **错误信封**：`{ "error": { "code": "STRING", "message": "STRING", "details": {} } }`

## 2. 通用错误码

| Code | HTTP | 何时 |
|---|---|---|
| INVALID_ARGUMENT | 400 | 请求参数不合法 |
| UNAUTHENTICATED | 401 | 未登录 |
| PERMISSION_DENIED | 403 | 无权 |
| NOT_FOUND | 404 | 资源不存在 |
| CONFLICT | 409 | 状态冲突 |
| RATE_LIMITED | 429 | 限流 |
| INTERNAL | 500 | 服务端错误 |

## 3. 端点清单

| 分组 | 方法 | 路径 | 用途 |
|---|---|---|---|
| Users | POST | `/api/v1/users` | 创建用户 |
| ... | ... | ... | ... |

## 4. 端点详细

### 4.1 `POST /api/v1/users` · 创建用户

**鉴权**：无（注册接口）

**请求**：
```json
{
  "email": "string",
  "password": "string",
  "display_name": "string"
}
```

**响应 · 201**：
```json
{
  "id": "uuid",
  "email": "string",
  "display_name": "string",
  "created_at": "ISO-8601"
}
```

**错误**：
- 400 INVALID_ARGUMENT · [具体触发条件]
- 409 CONFLICT · email 已存在

---

### 4.2 `GET /api/v1/<resource>/:id`

...

## 5. TypeScript 类型（前端可直接引用）

```typescript
export interface User {
  id: string;
  email: string;
  display_name: string;
  created_at: string;   // ISO
}

export interface ApiError {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}
```

## 6. 变更政策

- 加字段：向前兼容，直接发
- 改字段语义：破坏性 · 需版本号递增（`/api/v2/`）
- 删字段：先 deprecated 一版本，再删
