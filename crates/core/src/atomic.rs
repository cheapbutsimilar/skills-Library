//! JSON 원자적 저장: 임시 파일에 쓰고 rename해요. 직전 파일은 `.bak`으로 남겨요.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

pub fn backup_path(path: &Path) -> PathBuf {
    sibling(path, ".bak")
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = sibling(path, ".tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    if path.exists() {
        let _ = fs::copy(path, backup_path(path));
    }
    fs::rename(&tmp, path)
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    write_atomic(path, &bytes)
}

/// 본 파일이 깨졌으면 백업에서 읽어요. 둘 다 없거나 깨졌으면 `None`.
pub fn read_json_value(path: &Path) -> Option<serde_json::Value> {
    for p in [path.to_path_buf(), backup_path(path)] {
        if let Ok(bytes) = fs::read(&p) {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                return Some(v);
            }
        }
    }
    None
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    for p in [path.to_path_buf(), backup_path(path)] {
        if let Ok(bytes) = fs::read(&p) {
            if let Ok(v) = serde_json::from_slice::<T>(&bytes) {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;

    #[test]
    fn falls_back_to_backup_when_corrupt() {
        let t = TestDir::new("atomic");
        let p = t.path().join("s.json");
        write_json(&p, &serde_json::json!({"a": 1})).unwrap();
        write_json(&p, &serde_json::json!({"a": 2})).unwrap();
        fs::write(&p, "{ broken").unwrap();
        let v = read_json_value(&p).unwrap();
        assert_eq!(v["a"], 1);
    }
}
