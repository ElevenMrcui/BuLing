# providers/ · AI 能力来源适配器

每个 AI 能力来源（CLI / API / Local）对应一个子目录，实现统一的 `Provider` trait，供 `runtime/crates/opc-provider` 调度。

## 已规划的 Provider

### CLI Provider（走用户已安装的官方 CLI，最省钱最合规）

| 目录 | 底层 CLI | 适合什么 | P0? |
|---|---|---|---|
| `claude-code/` | `claude`（Anthropic 官方 Coding Agent）| 长上下文 · 代码 · 工具调用 · 文档 | ✅ |
| `codex/` | `codex`（OpenAI 官方 Codex CLI）| 代码 · 系统类任务 | ✅ |
| `gemini/` | `gemini`（Google Gemini CLI）| 长上下文 · Google 检索 | P1 |
| `aider/` | `aider` | 多文件编辑 · Git 集成 | P1 |

### API Provider（云端直连，用户配 Key）

| 目录 | 服务 | P0? |
|---|---|---|
| `anthropic/` | Anthropic Messages API | ✅ |
| `openai/` | OpenAI Chat Completions API | ✅ |
| `glm/` | 智谱 GLM | P1 |
| `qwen/` | 阿里通义 | P1 |
| `deepseek/` | DeepSeek | P1 |

### Local Provider（本机模型，端上完全私密）

| 目录 | 服务 | P0? |
|---|---|---|
| `ollama/` | Ollama（OpenAI 兼容协议）| ✅ |
| `lm-studio/` | LM Studio（OpenAI 兼容协议）| P1 |

## 统一 Provider Trait（Rust · 待 `opc-provider` crate 落地时定稿）

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> ProviderKind;             // Cli / Api / Local
    fn capabilities(&self) -> Capabilities;      // long-context, vision, tool-use, code, ...

    /// 执行一次调用
    async fn execute(&self, req: ProviderRequest) -> Result<ProviderResponse>;

    /// 流式调用（不支持则返回 Unimplemented）
    async fn stream(&self, req: ProviderRequest) -> Result<BoxStream<'static, StreamChunk>>;

    /// 取消一次进行中的调用
    async fn cancel(&self, run_id: &str) -> Result<()>;

    /// 探测可用状态
    async fn status(&self) -> Status;

    /// 计费（可选，用于 Cost 中心）
    fn cost(&self, usage: &Usage) -> Option<Money>;
}
```

## Adapter 目录规范

每个子目录应包含：

```
<provider>/
├── README.md                  该 Provider 的接入说明 · 已知限制 · 示例
├── manifest.toml              provider 元数据（kind / vendor / capabilities / 探测方法）
└── src/                       Rust adapter 源码（P0 之后加）
    └── lib.rs
```

**manifest.toml** 用于让 UI「模型中心」直接读，不需要跑 Rust。

## 与本地网关的关系

`apps/local-gateway`（旧仓库沿用）负责**发现**：在用户机器上扫描哪些 CLI 装了 / 版本多少。发现结果供 UI 展示 + Provider Adapter 决定用什么参数调用。

**未来 P0 后期**：`local-gateway` 从"独立 daemon"融入 `runtime/crates/opc-provider`——不再需要单独跑一个进程，Tauri App 启动时 in-process 完成扫描。旧的 daemon 形态仍保留（用于给非 OPC 的其他工具挂钩本机 CLI）。

## 关联文档

- `docs/OPC-产品定义.md § 8` —— Provider 抽象
- `docs/OPC-产品定义.md § 9` —— 一分钟组阁流程
- `packages/cli-registry` —— CLI 签名注册表（发现层）
- `apps/local-gateway` —— 本机 daemon（当前形态）
