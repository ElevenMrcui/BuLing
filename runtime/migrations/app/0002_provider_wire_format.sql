-- ============================================================================
-- 0002_provider_wire_format —— 给 providers 加线协议字段
--
-- 背景：opc-provider crate 落地时（见 docs/OPC-架构决策.md ADR-005 附注）
-- api/local 类型的 Provider 需要知道走哪种 HTTP wire format 才能调用：
--   · anthropic-messages —— Anthropic 官方 Messages API
--   · openai-compatible  —— OpenAI / GLM / Qwen / DeepSeek / Ollama / LM Studio
--     等厂商声明兼容的 Chat Completions 形状
--
-- cli 类型的 Provider 不需要这个字段（走 cli_binary + 对应 CliAdapter），
-- 所以允许为 NULL。
-- ============================================================================

ALTER TABLE providers ADD COLUMN wire_format TEXT
    CHECK (wire_format IS NULL OR wire_format IN ('anthropic-messages', 'openai-compatible'));

CREATE INDEX providers_wire_format_idx ON providers (wire_format);
