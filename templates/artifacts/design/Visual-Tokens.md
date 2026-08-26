---
kind: visual-tokens
version: 1
producer: 设计师
inputs:
  - ux-overview
project: <auto-fill>
created_at: <auto-fill-iso>
---

# 视觉规范 · Visual Tokens

<!--
设计师产出。守则：
1. 只定义 **tokens**（颜色 / 字号 / 间距 / 圆角 / 阴影）· 不做详细样式
2. 三态主题：浅色 · 深色 · 跟随系统；深色不是"简单反色"，各角色独立微调
3. 颜色不作为唯一语义线索（配图标 / 文案）
-->

## 1. 颜色

### 1.1 中性色（浅色）

| Token | Hex | 用途 |
|---|---|---|
| `--bg` | #f5f5f7 | 页面背景 |
| `--bg-elevated` | #fbfbfd | 卡片背景 |
| `--surface` | #ffffff | 组件表面 |
| `--surface-sunken` | #eeeef0 | 输入 / 沉降 |
| `--ink` | #1d1d1f | 正文 |
| `--ink-secondary` | #494952 | 次要文本 |
| `--ink-tertiary` | #8e8e93 | 辅助文本 |
| `--border` | #e5e5ea | 边框 |
| `--border-strong` | #d1d1d6 | 强边框 |

### 1.2 中性色（深色）

| Token | Hex |
|---|---|
| `--bg` | #0a0a0d |
| `--bg-elevated` | #14141a |
| `--surface` | #1c1c22 |
| `--ink` | #f5f5f7 |
| ... | ... |

### 1.3 品牌 / 强调色

| Token | 浅色 Hex | 深色 Hex | 用途 |
|---|---|---|---|
| `--accent` | [#007aff] | [#0a84ff] | 主 CTA |
| `--accent-soft` | [#e5f0ff] | [#003366] | 主 CTA hover 底 |

### 1.4 状态色

| Token | 语义 | Hex |
|---|---|---|
| `--success` | 成功 | [#34c759] |
| `--warning` | 警告 | [#ff9500] |
| `--danger` | 错误 / 破坏 | [#ff3b30] |
| `--info` | 提示 | [#5ac8fa] |

## 2. 字号

| Token | 值 | 用途 |
|---|---|---|
| `--text-xs` | 11px | 元信息 |
| `--text-sm` | 12.5px | 辅助 |
| `--text-base` | 14px | 正文 |
| `--text-lg` | 16px | 强调 |
| `--text-xl` | 19px | 小标题 |
| `--text-2xl` | 22px | 页面标题 |
| `--text-3xl` | 28px | 关键数字 / Hero |

**字体**：
- 中文：`-apple-system, "PingFang SC", "Noto Sans SC", ...`
- 英文：`-apple-system, "SF Pro Text", "Inter", ...`
- 数字：`font-variant-numeric: tabular-nums`（对齐）

## 3. 间距 / 圆角 / 阴影

**间距（4 的倍数）**：
| Token | 值 |
|---|---|
| `--space-1` | 4px |
| `--space-2` | 8px |
| `--space-3` | 12px |
| `--space-4` | 16px |
| `--space-6` | 24px |
| `--space-8` | 32px |

**圆角**：
| Token | 值 |
|---|---|
| `--radius-s` | 6px |
| `--radius-m` | 10px |
| `--radius-l` | 16px |
| `--radius-full` | 9999px |

**阴影**：
| Token | 值 |
|---|---|
| `--shadow-sm` | 0 1px 2px rgba(0,0,0,.06) |
| `--shadow-md` | 0 4px 12px rgba(0,0,0,.08) |
| `--shadow-lg` | 0 12px 40px rgba(0,0,0,.12) |

## 4. 主题合规

- 所有颜色**先在 `:root` 定义完整浅色一份**
- `@media (prefers-color-scheme: dark)` 里再改
- `[data-theme="dark"]` / `[data-theme="light"]` 支持显式覆盖
- **禁止把颜色只定义在 `@media` 或 `[data-theme]` 块里**（会导致某个状态下颜色缺失）

## 5. 图标

- 图标风格：[线性 / 填充]
- 尺寸：16 · 20 · 24
- 颜色：继承 `currentColor`
