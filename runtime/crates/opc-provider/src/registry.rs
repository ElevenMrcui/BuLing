//! Factory：把 `ProviderManifest` 变成一个可用的 `Box<dyn Provider>`。
//! Runtime 只应通过这里拿 Provider 实例，不直接 `new` 具体类型。

use std::collections::HashMap;
use std::path::Path;

use crate::api::ApiProvider;
use crate::cli::CliProvider;
use crate::error::Result;
use crate::manifest::{load_manifests_from_dir, ProviderManifest};
use crate::provider::Provider;
use crate::types::ProviderKind;

pub struct ProviderRegistry {
    manifests: HashMap<String, ProviderManifest>,
}

impl ProviderRegistry {
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let list = load_manifests_from_dir(dir)?;
        let manifests = list.into_iter().map(|m| (m.id.clone(), m)).collect();
        Ok(Self { manifests })
    }

    pub fn manifest(&self, id: &str) -> Option<&ProviderManifest> {
        self.manifests.get(id)
    }

    pub fn manifests(&self) -> impl Iterator<Item = &ProviderManifest> {
        self.manifests.values()
    }

    /// 构建一个 Provider 实例。`base_url_override` 仅对 api/local 类型有意义
    /// （用户在「模型中心」自定义了 base_url 时传入）。
    pub fn build(&self, id: &str, base_url_override: Option<String>) -> Result<Box<dyn Provider>> {
        let manifest = self
            .manifest(id)
            .ok_or_else(|| crate::error::Error::Manifest { file: id.to_string(), reason: "unknown provider id".into() })?
            .clone();

        match manifest.kind {
            ProviderKind::Cli => Ok(Box::new(CliProvider::from_manifest(manifest)?)),
            ProviderKind::Api | ProviderKind::Local => {
                Ok(Box::new(ApiProvider::from_manifest(manifest, base_url_override)?))
            }
        }
    }
}
