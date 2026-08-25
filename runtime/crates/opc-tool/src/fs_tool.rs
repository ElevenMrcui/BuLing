//! 项目内文件写入（沙箱化）—— 对齐 `permission_defaults.file = "project-only"`
//! （见 `agents/README.md`）：Agent 只能写自己项目目录以内的文件，不管
//! 传进来的相对路径里有多少个 `..`，也不管是不是绝对路径。

use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// 词法归一化（不要求路径真实存在——写入场景里父目录经常还没建）：
/// 逐段处理 `.` / `..` / 正常段，`..` 就 pop 掉上一段。
fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// 把一个"项目根相对路径"解析成绝对路径，并校验它落在 `project_root` 以内。
///
/// 拒绝：
/// - 绝对路径（`/etc/passwd`）
/// - 用 `..` 穿越出项目根（`../../etc/passwd`）
pub fn resolve_project_path(project_root: &Path, relative: &str) -> Result<PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err(Error::PathEscape { path: relative.to_string() });
    }
    let root_normalized = normalize_lexical(project_root);
    let joined_normalized = normalize_lexical(&project_root.join(rel));
    if !joined_normalized.starts_with(&root_normalized) {
        return Err(Error::PathEscape { path: relative.to_string() });
    }
    Ok(joined_normalized)
}

pub struct WrittenFile {
    pub abs_path: PathBuf,
    pub file_hash: String,
    pub file_bytes: i64,
}

/// 写一段文本到项目内相对路径，自动建父目录；返回 sha256 与字节数供
/// Artifact 登记使用。
pub async fn write_text_file(project_root: &Path, relative_path: &str, content: &str) -> Result<WrittenFile> {
    let abs = resolve_project_path(project_root, relative_path)?;
    if let Some(parent) = abs.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&abs, content.as_bytes()).await?;

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let file_hash = format!("{:x}", hasher.finalize());

    Ok(WrittenFile { abs_path: abs, file_hash, file_bytes: content.len() as i64 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_normal_relative_path() {
        let root = Path::new("/home/user/proj");
        let p = resolve_project_path(root, "docs/product/PRD.md").unwrap();
        assert_eq!(p, PathBuf::from("/home/user/proj/docs/product/PRD.md"));
    }

    #[test]
    fn rejects_absolute_path() {
        let root = Path::new("/home/user/proj");
        let err = resolve_project_path(root, "/etc/passwd").unwrap_err();
        assert!(matches!(err, Error::PathEscape { .. }));
    }

    #[test]
    fn rejects_traversal_out_of_root() {
        let root = Path::new("/home/user/proj");
        let err = resolve_project_path(root, "../../etc/passwd").unwrap_err();
        assert!(matches!(err, Error::PathEscape { .. }));
    }

    #[test]
    fn allows_traversal_that_stays_inside_root() {
        let root = Path::new("/home/user/proj");
        // docs/../docs/product/PRD.md 词法归一化后还是落在 root 内
        let p = resolve_project_path(root, "docs/../docs/product/PRD.md").unwrap();
        assert_eq!(p, PathBuf::from("/home/user/proj/docs/product/PRD.md"));
    }
}
