# providers/ · AI 能力来源 manifest

每个 AI 能力来源（CLI / API / Local）对应一个子目录，放一份 `manifest.toml`，供 `runtime/crates/opc-provider` 的 `ProviderRegistry` 加载。**新增一家走已知协议的厂商只需要加一份 TOML，不需要碰 Rust 代码**（Factory + 声明式 manifest，见 `docs/OPC-架构决策.md` ADR-005 附注）。

## Provider 清单（✅ = manifest 已就绪且可构建实例；⚠️ = manifest 已就绪但 adapter 未实现）

### CLI Provider（走用户已安装的官方 CLI，最省钱最合规）

| 目录 | id | 底层 CLI | 状态 |
|---|---|---|---|
| `claude-code/` | `claude-code` | `claude`（Anthropic 官方 Coding Agent）| ✅ `ClaudeCodeAdapter` 已实现，参数经真实 `claude --help` 核实 |
| `codex/` | `codex-cli` | `codex`（OpenAI 官方 Codex CLI）| ⚠️ manifest 占位，adapter 未实现（未核实真实参数，不臆造） |
| `gemini/` | `gemini-cli` | `gemini`（Google Gemini CLI）| ⚠️ 同上 |
| `aider/` | `aider` | `aider` | ⚠️ 同上 |

### API Provider（云端直连，用户配 Key，走 OS Keychain）

| 目录 | id | wire_format | 状态 |
|---|---|---|---|
| `anthropic/` | `anthropic-api` | `anthropic-messages` | ✅ 字段用 claude-api skill 权威参考核实 |
| `openai/` | `openai-api` | `openai-compatible` | ✅ |
| `glm/` | `glm-api` | `openai-compatible` | ✅（智谱官方文档声明兼容） |
| `qwen/` | `qwen-api` | `openai-compatible` | ✅（DashScope 兼容模式） |
| `deepseek/` | `deepseek-api` | `openai-compatible` | ✅（DeepSeek 官方文档声明兼容） |

### Local Provider（本机模型，端上完全私密，`allowed_sensitivity` 含 `high`）

| 目录 | id | wire_format | 状态 |
|---|---|---|---|
| `ollama/` | `ollama-local` | `openai-compatible` | ✅ |
| `lm-studio/` | `lm-studio-local` | `openai-compatible` | ✅ |

## `Provider` Trait（已在 `runtime/crates/opc-provider/src/provider.rs` 落地）

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> ProviderKind;              // Cli / Api / Local

    /// 执行一次文本补全（system + 历史进，文本 + usage 出）。
    /// 不做工具调用循环——那是 Runtime（opc-tool + opc-workflow）的职责。
    async fn execute(&self, req: &ProviderRequest) -> Result<ProviderResponse>;

    /// 探测可用性：CLI 是否装了 / API Key 是否能取到。不发起真实推理请求。
    async fn status(&self) -> ProviderStatusReport;
}
```

**P0 刻意不做**（避免 Provider 变成什么都干的上帝对象）：流式响应、取消令牌、内建计费——这些留到真正需要时再加，不预先设计。

## manifest.toml 字段规范（对齐 `runtime/crates/opc-provider/src/manifest.rs`）

```toml
id = "anthropic-api"              # 稳定 id，对齐 app.sqlite providers 表主键
display_name = "Anthropic 官方"
vendor = "Anthropic"
kind = "api"                      # cli | api | local

# api/local 用：走哪种 wire format
wire_format = "anthropic-messages"  # anthropic-messages | openai-compatible

# cli 用：走哪个 CliAdapter 实现 + 可执行文件名
# cli_adapter = "claude-code"
# cli_binary = "claude"

default_base_url = "https://api.anthropic.com"
credential_service = "anthropic"  # OS Keychain service 后缀：opc.provider.<this>
default_model = "claude-sonnet-5"
models = ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"]

allowed_sensitivity = ["low", "medium"]   # 决定哪些 Agent.sensitivity 能绑这个 Provider
privacy_note = "一句话隐私声明，UI 展示用"
```

`kind = "cli"` 的 manifest 忽略 `wire_format` / `default_base_url` / `credential_service`；`kind = "api"/"local"` 的 manifest 忽略 `cli_adapter` / `cli_binary`。

## 目录规范

```
<provider>/
└── manifest.toml    # 唯一必需文件；本目录本身不含 Rust 源码
```

Rust 实现集中在 `runtime/crates/opc-provider/src/{api,cli}/`：`WireFormat` 实现放 `api/`，`CliAdapter` 实现放 `cli/`。这里的子目录只放**声明式元数据**，不放代码——保持"加厂商 = 加 TOML"的开闭原则。

## API Key 安全

绝不落 SQLite。`credential_service` 决定 OS Keychain 的 service 名（`opc.provider.<credential_service>`，account 固定 `default`）；`runtime/crates/opc-provider/src/credential.rs` 负责读写，环境变量 `OPC_KEY_<SERVICE>`（大写，`-`/`.` 转 `_`）作为 CI / 无头环境兜底。详见 `docs/OPC-架构决策.md` ADR-004。

## 与本地网关的关系

`apps/local-gateway`（旧仓库沿用）负责**发现**：在用户机器上扫描哪些 CLI 装了 / 版本多少。发现结果供 UI 展示；实际调用走 `opc-provider` 的 `CliProvider`。

**未来 P0 后期**：`local-gateway` 从"独立 daemon"融入 `runtime/crates/opc-provider`——不再需要单独跑一个进程，Tauri App 启动时 in-process 完成扫描。旧的 daemon 形态仍保留（用于给非 OPC 的其他工具挂钩本机 CLI）。

## 加一家新厂商的步骤

1. **走已支持的协议**（`openai-compatible` / `anthropic-messages`）：只需加一份 `manifest.toml`，跑 `cargo test -p opc-provider` 确认 `manifest_loading_covers_expected_providers` 之类的测试认得它
2. **走新的 CLI**：先用真实 `<cli> --help` 核实参数（不臆造），实现一个新的 `CliAdapter`，在 `cli/mod.rs` 的 `from_manifest` match 里加一个分支
3. **走新的 API 协议**：实现一个新的 `WireFormat`，在 `manifest.rs` 的 `WireFormat` enum 加一个变体，在 `api/mod.rs` 的 `from_manifest` match 里加一个分支

## 关联文档

- `docs/OPC-产品定义.md § 8` —— Provider 抽象（产品视角）
- `docs/OPC-产品定义.md § 9` —— 一分钟组阁流程
- `docs/OPC-架构决策.md` ADR-004（Key 安全）· ADR-005（Provider 抽象 + 落地附注）
- `docs/OPC-数据模型.md` —— `app.sqlite.providers` 表字段（含 `wire_format` 列）
- `runtime/crates/opc-provider/` —— 实现源码
- `packages/cli-registry` —— CLI 签名注册表（发现层）
- `apps/local-gateway` —— 本机 daemon（当前形态）
