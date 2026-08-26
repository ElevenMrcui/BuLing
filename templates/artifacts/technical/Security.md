---
kind: security
version: 1
producer: 架构师
inputs:
  - prd
  - architecture
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 安全设计 · Security

<!--
架构师产出。硬约束：
1. 不允许把安全塞进"通用架构"章节，必须独立成文
2. 每条威胁配"防御"和"验证方式"
3. API Key / 用户密码 / PII 处理必须显式写
-->

## 1. 威胁模型

| 威胁 | 攻击者 | 攻击路径 | 影响 | 缓解 |
|---|---|---|---|---|
| [XSS] | 匿名用户 | 恶意评论注入 script | 会话劫持 | React 自动转义 + CSP |
| ... | ... | ... | ... | ... |

## 2. 鉴权

**方案**：[JWT · OAuth 2.0 · Session Cookie · ...]

**Token 生命周期**：
- Access Token：[TTL]，[存储位置]
- Refresh Token：[TTL]，[存储位置]

**密码存储**：
- 算法：[bcrypt · argon2id]
- 参数：[cost / memory]

## 3. 授权

**模型**：[RBAC · ABAC · owner-only · ...]

**权限矩阵**：

| 角色 | 资源 | 动作 |
|---|---|---|
| owner | posts | CRUD |
| viewer | posts | R |

## 4. 敏感数据处理

| 数据 | 存储 | 加密 | 传输 | 访问日志 |
|---|---|---|---|---|
| 密码 | DB | bcrypt hash | TLS | 否 |
| API Key | OS Keychain | 系统 | TLS | 是 |
| PII 邮箱 | DB | 无（明文）| TLS | 是 |

## 5. 传输安全

- TLS 1.3 强制
- HSTS: `max-age=31536000; includeSubDomains`
- Cookie: `Secure; HttpOnly; SameSite=Strict`

## 6. 输入验证

- 所有外部输入过 schema 校验（zod / joi / pydantic）
- 拒绝深嵌套 JSON（防 CPU DoS）
- 上传文件白名单 MIME + 大小限制

## 7. 日志与审计

- 所有鉴权动作写审计日志（登录 / 授权变更 / 高风险操作）
- 日志中**禁止**出现密码 / Token / API Key 明文
- 保留 [N] 天，之后归档 / 删除

## 8. 依赖安全

- 每周跑 `npm audit` / `cargo audit` / `pip-audit`
- 关键库锁死小版本，重大升级走 CVE 审阅

## 9. 待用户拍板

- [ ] 是否上 CSP nonce（严格模式）
- [ ] 是否强制 MFA
