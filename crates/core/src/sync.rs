//! 설치 기록(manifest)과 동기화 상태.
//!
//! 설치할 때 내용 해시를 적어 두면, 나중에 라이브러리·설치본 중 어느 쪽이 바뀌었는지 알 수 있어요.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Record {
    pub hash: String,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Manifest {
    pub v: u32,
    /// 대상 id → 스킬 폴더 이름 → 마지막으로 설치(또는 내보내기)한 내용
    pub installs: BTreeMap<String, BTreeMap<String, Record>>,
    /// 이미 알린 "새 외부 스킬" (`대상id/폴더`)
    pub notified: BTreeSet<String>,
    /// 첫 검사를 마쳤는지 (첫 실행 때 이미 있던 스킬로 알림을 쏟아내지 않게)
    pub notified_init: bool,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest { v: 1, installs: BTreeMap::new(), notified: BTreeSet::new(), notified_init: false }
    }
}

impl Manifest {
    pub fn get(&self, target: &str, slug: &str) -> Option<&Record> {
        self.installs.get(target).and_then(|m| m.get(slug))
    }

    /// 바뀌었으면 true
    pub fn set(&mut self, target: &str, slug: &str, hash: &str, at: u64) -> bool {
        let m = self.installs.entry(target.into()).or_default();
        if m.get(slug).map(|r| r.hash.as_str()) == Some(hash) {
            return false;
        }
        m.insert(slug.into(), Record { hash: hash.into(), at });
        true
    }

    pub fn remove(&mut self, target: &str, slug: &str) -> bool {
        let changed = self.installs.get_mut(target).map(|m| m.remove(slug).is_some()).unwrap_or(false);
        if self.installs.get(target).is_some_and(|m| m.is_empty()) {
            self.installs.remove(target);
        }
        changed
    }

    pub fn remove_target(&mut self, target: &str) {
        self.installs.remove(target);
        let prefix = format!("{target}/");
        self.notified.retain(|k| !k.starts_with(&prefix));
    }

    pub fn targets_with(&self, slug: &str) -> Vec<String> {
        self.installs.iter().filter(|(_, m)| m.contains_key(slug)).map(|(t, _)| t.clone()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// 설치 위치에 없어요
    NotInstalled,
    /// 라이브러리와 같아요
    Synced,
    /// 라이브러리가 바뀌었어요 → 업데이트
    LibraryNewer,
    /// 설치 위치에서 누가 고쳤어요 → 가져오기 또는 덮어쓰기
    TargetModified,
    /// 양쪽 다 바뀌었어요
    Conflict,
    /// 이름만 같은, 이 앱이 설치하지 않은 다른 내용이에요
    Foreign,
}

impl Status {
    pub fn installed(self) -> bool {
        !matches!(self, Status::NotInstalled)
    }
    pub fn needs_attention(self) -> bool {
        matches!(self, Status::TargetModified | Status::Conflict | Status::Foreign)
    }
}

pub fn status(lib_hash: &str, target_hash: Option<&str>, record: Option<&str>) -> Status {
    match (target_hash, record) {
        (None, _) => Status::NotInstalled,
        (Some(t), _) if t == lib_hash => Status::Synced,
        (Some(t), Some(r)) if t == r => Status::LibraryNewer,
        (Some(_), Some(r)) if r == lib_hash => Status::TargetModified,
        (Some(_), Some(_)) => Status::Conflict,
        (Some(_), None) => Status::Foreign,
    }
}

/// Claude 앱처럼 내보내기만 하는 곳: 내보낸 뒤 라이브러리가 바뀌었는지만 봐요.
pub fn export_status(lib_hash: &str, record: Option<&str>) -> Status {
    match record {
        None => Status::NotInstalled,
        Some(r) if r == lib_hash => Status::Synced,
        Some(_) => Status::LibraryNewer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_table() {
        assert_eq!(status("L", None, None), Status::NotInstalled);
        assert_eq!(status("L", Some("L"), None), Status::Synced);
        assert_eq!(status("L", Some("L"), Some("old")), Status::Synced);
        assert_eq!(status("L", Some("R"), Some("R")), Status::LibraryNewer);
        assert_eq!(status("L", Some("T"), Some("L")), Status::TargetModified);
        assert_eq!(status("L", Some("T"), Some("R")), Status::Conflict);
        assert_eq!(status("L", Some("T"), None), Status::Foreign);
        assert_eq!(export_status("L", Some("old")), Status::LibraryNewer);
    }

    #[test]
    fn manifest_set_remove() {
        let mut m = Manifest::default();
        assert!(m.set("t", "s", "h", 1));
        assert!(!m.set("t", "s", "h", 2));
        assert_eq!(m.targets_with("s"), vec!["t".to_string()]);
        assert!(m.remove("t", "s"));
        assert!(m.installs.is_empty());
    }
}
