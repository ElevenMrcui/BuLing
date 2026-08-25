//! 把 `agents/*.yaml` 播种进 `app.sqlite` 的 `agents` 表。
//!
//! 幂等 upsert：
//! - 首次遇到某 id → INSERT
//! - 已存在且 `kind='preset'` → UPDATE（只有 `system_prompt` 变化时才递增
//!   `version`，避免"内容没变但每次启动都 +1"的版本号噪音）
//! - 已存在但 `kind='user'`（用户 fork 过的同名？理论不该发生，id 生成策略
//!   保证不撞）→ **不覆盖**，`ON CONFLICT ... WHERE kind='preset'` 天然跳过

use opc_storage::AppDb;

use crate::definition::AgentDefinition;
use crate::error::Result;

pub async fn seed_agents(db: &AppDb, defs: &[AgentDefinition]) -> Result<usize> {
    let mut count = 0usize;
    for def in defs {
        let responsibilities = serde_json::to_string(&def.responsibilities)?;
        let expected_outputs = serde_json::to_string(&def.expected_outputs)?;
        let capabilities = serde_json::to_string(&def.capabilities)?;
        let skills = serde_json::to_string(&def.skills)?;
        let tools = serde_json::to_string(&def.tools)?;
        let mcp_servers = serde_json::to_string(&def.mcp_servers)?;
        let provider_priority = serde_json::to_string(&def.provider_priority)?;
        let permission_defaults = serde_json::to_string(&def.permission_defaults)?;

        sqlx::query(
            r#"
            INSERT INTO agents (
                id, kind, role, display_name, avatar, system_prompt,
                responsibilities, expected_outputs, capabilities, sensitivity,
                skills, tools, mcp_servers, provider_priority, permission_defaults,
                updated_at
            )
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                role                 = excluded.role,
                display_name         = excluded.display_name,
                avatar                = excluded.avatar,
                system_prompt         = excluded.system_prompt,
                responsibilities      = excluded.responsibilities,
                expected_outputs      = excluded.expected_outputs,
                capabilities           = excluded.capabilities,
                sensitivity            = excluded.sensitivity,
                skills                  = excluded.skills,
                tools                    = excluded.tools,
                mcp_servers               = excluded.mcp_servers,
                provider_priority          = excluded.provider_priority,
                permission_defaults          = excluded.permission_defaults,
                version = CASE WHEN agents.system_prompt != excluded.system_prompt
                               THEN agents.version + 1
                               ELSE agents.version END,
                updated_at = datetime('now')
            WHERE agents.kind = 'preset'
            "#,
        )
        .bind(&def.id)
        .bind(&def.kind)
        .bind(&def.role)
        .bind(&def.display_name)
        .bind(&def.avatar)
        .bind(&def.system_prompt)
        .bind(responsibilities)
        .bind(expected_outputs)
        .bind(capabilities)
        .bind(&def.sensitivity)
        .bind(skills)
        .bind(tools)
        .bind(mcp_servers)
        .bind(provider_priority)
        .bind(permission_defaults)
        .execute(&db.pool)
        .await?;
        count += 1;
    }
    Ok(count)
}
