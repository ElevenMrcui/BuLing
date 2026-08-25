//! API Key 解析（见 `docs/OPC-架构决策.md` ADR-004）：优先 OS Keychain，
//! CI / 无头环境用环境变量兜底。**永不落 SQLite**。

use crate::error::{Error, Result};

const KEYCHAIN_SERVICE_PREFIX: &str = "opc.provider";

/// 解析某个 Provider 的 API Key。
///
/// 解析顺序：
/// 1. 环境变量 `OPC_KEY_<SERVICE_UPPER>`（CI / 无头环境 / 开发期兜底）
/// 2. OS Keychain：service = `opc.provider.<service>`，account = `default`
pub fn resolve_api_key(credential_service: &str) -> Result<String> {
    let env_key = format!(
        "OPC_KEY_{}",
        credential_service.to_uppercase().replace(['-', '.'], "_")
    );
    if let Ok(v) = std::env::var(&env_key) {
        if !v.is_empty() {
            return Ok(v);
        }
    }

    let service = format!("{KEYCHAIN_SERVICE_PREFIX}.{credential_service}");
    let entry = keyring::Entry::new(&service, "default")?;
    match entry.get_password() {
        Ok(pw) => Ok(pw),
        Err(keyring::Error::NoEntry) => Err(Error::CredentialMissing {
            provider: credential_service.to_string(),
            reason: format!(
                "既没有环境变量 {env_key}，也没有 Keychain 条目 {service}/default；\
                 请在「模型中心」配置该 Provider 的 API Key"
            ),
        }),
        Err(e) => Err(e.into()),
    }
}

/// 写入某个 Provider 的 API Key 到 OS Keychain（模型中心「保存」按钮调用）。
pub fn store_api_key(credential_service: &str, api_key: &str) -> Result<()> {
    let service = format!("{KEYCHAIN_SERVICE_PREFIX}.{credential_service}");
    let entry = keyring::Entry::new(&service, "default")?;
    entry.set_password(api_key)?;
    Ok(())
}

/// 从 OS Keychain 删除某个 Provider 的 API Key（用户断开连接时调用）。
pub fn delete_api_key(credential_service: &str) -> Result<()> {
    let service = format!("{KEYCHAIN_SERVICE_PREFIX}.{credential_service}");
    let entry = keyring::Entry::new(&service, "default")?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
