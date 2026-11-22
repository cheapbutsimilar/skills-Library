//! 파일 시스템 도우미: 폴더 목록, 내용 해시, 복사, 휴지통으로 옮기기.

use sha2::{Digest, Sha256};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 스킬 내용으로 치지 않는 파일·폴더 이름.
pub const IGNORED: &[&str] = &[
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",
    ".git",
    "__pycache__",
    "node_modules",
];

/// 한 스킬 폴더에서 읽을 최대 파일 수 (실수로 큰 폴더를 가리켰을 때 멈추지 않게).
pub const MAX_FILES: usize = 5000;
const MAX_DEPTH: usize = 12;

pub fn is_ignored(name: &str) -> bool {
    IGNORED.contains(&name)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn mtime_ms(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 폴더 안 파일의 상대 경로('/' 구분)를 정렬해서 돌려줘요.
/// 폴더를 가리키는 심볼릭 링크는 따라가지 않아요(순환 방지). 파일 링크는 따라가요.
pub fn list_files(dir: &Path) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    walk(dir, "", &mut out, 0)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>, depth: usize) -> io::Result<()> {
    if depth > MAX_DEPTH || out.len() >= MAX_FILES {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if is_ignored(&name) {
            continue;
        }
        let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk(&entry.path(), &rel, out, depth + 1)?;
        } else if ft.is_symlink() {
            match fs::metadata(entry.path()) {
                Ok(m) if m.is_file() => out.push(rel),
                _ => {}
            }
        } else if ft.is_file() {
            out.push(rel);
        }
        if out.len() >= MAX_FILES {
            break;
        }
    }
    Ok(())
}

pub fn join_rel(dir: &Path, rel: &str) -> PathBuf {
    let mut p = dir.to_path_buf();
    for part in rel.split('/') {
        p.push(part);
    }
    p
}

/// 상대 경로가 폴더 밖으로 나가지 않는지 검사해요.
pub fn safe_rel(rel: &str) -> bool {
    let rel = rel.trim();
    if rel.is_empty() || rel.len() > 400 || rel.starts_with('/') || rel.starts_with('\\') || rel.contains(':') {
        return false;
    }
    rel.split(['/', '\\']).all(|p| !p.is_empty() && p != "." && p != "..")
}

/// 폴더 내용 해시 (상대 경로 + 크기 + 바이트). 같은 내용이면 어느 위치에 있든 같아요.
pub fn hash_dir(dir: &Path) -> io::Result<String> {
    let files = list_files(dir)?;
    let mut h = Sha256::new();
    for rel in &files {
        let bytes = fs::read(join_rel(dir, rel))?;
        h.update(rel.as_bytes());
        h.update([0u8]);
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(&bytes);
    }
    let digest = h.finalize();
    Ok(digest[..16].iter().map(|b| format!("{b:02x}")).collect())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DirStats {
    pub files: usize,
    pub bytes: u64,
    pub updated: u64,
}

pub fn dir_stats(dir: &Path) -> DirStats {
    let mut s = DirStats::default();
    if let Ok(files) = list_files(dir) {
        for rel in files {
            if let Ok(m) = fs::metadata(join_rel(dir, &rel)) {
                s.files += 1;
                s.bytes += m.len();
                s.updated = s.updated.max(mtime_ms(&m));
            }
        }
    }
    s
}

/// `src` 폴더 내용을 `dst`에 새로 만들어요. `dst`가 있으면 통째로 바꿔요.
/// 임시 폴더에 다 복사한 다음 바꿔치기해서, 중간에 실패해도 원래 폴더가 남아요.
pub fn replace_dir(src: &Path, dst: &Path) -> io::Result<()> {
    let parent = dst.parent().ok_or_else(|| io::Error::other("상위 폴더가 없어요"))?;
    fs::create_dir_all(parent)?;
    let name = dst.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let stamp = now_ms();
    let tmp = parent.join(format!(".{name}.tmp-{stamp}"));
    let old = parent.join(format!(".{name}.old-{stamp}"));
    if let Err(e) = copy_tree(src, &tmp) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    let had_old = dst.exists() || fs::symlink_metadata(dst).is_ok();
    if had_old {
        if let Err(e) = fs::rename(dst, &old) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
    }
    if let Err(e) = fs::rename(&tmp, dst) {
        if had_old {
            let _ = fs::rename(&old, dst);
        }
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    if had_old {
        remove_any(&old)?;
    }
    Ok(())
}

fn remove_any(p: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(p)?;
    if meta.is_dir() {
        fs::remove_dir_all(p)
    } else {
        fs::remove_file(p)
    }
}

/// 무시 목록을 빼고 폴더를 복사해요.
pub fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for rel in list_files(src)? {
        let from = join_rel(src, &rel);
        let to = join_rel(dst, &rel);
        if let Some(p) = to.parent() {
            fs::create_dir_all(p)?;
        }
        fs::copy(&from, &to)?;
    }
    Ok(())
}

/// 지우는 대신 휴지통 폴더(`trash/<시각>-<label>/<이름>`)로 옮겨요.
pub fn move_to_trash(path: &Path, trash_root: &Path, label: &str) -> io::Result<PathBuf> {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "item".into());
    let label: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .take(40)
        .collect();
    let bucket = trash_root.join(format!("{}-{}", now_ms(), label));
    fs::create_dir_all(&bucket)?;
    let dest = bucket.join(&name);
    match fs::rename(path, &dest) {
        Ok(()) => Ok(dest),
        Err(_) => {
            // 다른 드라이브면 rename이 안 돼요 → 복사 후 삭제.
            let meta = fs::symlink_metadata(path)?;
            if meta.is_dir() {
                copy_tree(path, &dest)?;
            } else {
                fs::copy(path, &dest)?;
            }
            remove_any(path)?;
            Ok(dest)
        }
    }
}

/// 변경 감지용 값싼 지문: 경로·크기·수정 시각만 봐요 (내용은 읽지 않아요).
pub fn fingerprint(roots: &[PathBuf]) -> u64 {
    let mut h = DefaultHasher::new();
    for root in roots {
        root.hash(&mut h);
        fp_walk(root, &mut h, 0);
    }
    h.finish()
}

fn fp_walk(dir: &Path, h: &mut DefaultHasher, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else {
        0u8.hash(h);
        return;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries.into_iter().take(4000) {
        let name = e.file_name();
        if is_ignored(&name.to_string_lossy()) {
            continue;
        }
        name.hash(h);
        if let Ok(m) = fs::metadata(e.path()) {
            m.len().hash(h);
            mtime_ms(&m).hash(h);
            if m.is_dir() {
                fp_walk(&e.path(), h, depth + 1);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod testdir {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    pub struct TestDir(pub PathBuf);
    impl TestDir {
        pub fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!(
                "skillslib-test-{}-{}-{}",
                tag,
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            TestDir(p)
        }
        pub fn path(&self) -> &Path {
            &self.0
        }
        pub fn write(&self, rel: &str, content: &str) -> PathBuf {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, content).unwrap();
            p
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testdir::TestDir;
    use super::*;

    #[test]
    fn hash_is_location_independent_and_content_sensitive() {
        let t = TestDir::new("hash");
        t.write("a/SKILL.md", "hello");
        t.write("a/ref/x.md", "x");
        t.write("a/.DS_Store", "junk");
        t.write("b/SKILL.md", "hello");
        t.write("b/ref/x.md", "x");
        let ha = hash_dir(&t.path().join("a")).unwrap();
        let hb = hash_dir(&t.path().join("b")).unwrap();
        assert_eq!(ha, hb);
        t.write("b/ref/x.md", "y");
        assert_ne!(ha, hash_dir(&t.path().join("b")).unwrap());
    }

    #[test]
    fn replace_dir_swaps_contents() {
        let t = TestDir::new("replace");
        t.write("src/SKILL.md", "new");
        t.write("dst/old.md", "old");
        replace_dir(&t.path().join("src"), &t.path().join("dst")).unwrap();
        assert!(!t.path().join("dst/old.md").exists());
        assert_eq!(fs::read_to_string(t.path().join("dst/SKILL.md")).unwrap(), "new");
        // 임시 폴더가 남지 않아요.
        let leftovers: Vec<_> = fs::read_dir(t.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn trash_moves_instead_of_deleting() {
        let t = TestDir::new("trash");
        t.write("s/SKILL.md", "x");
        let dest = move_to_trash(&t.path().join("s"), &t.path().join("trash"), "lib/s").unwrap();
        assert!(!t.path().join("s").exists());
        assert!(dest.join("SKILL.md").exists());
    }

    #[test]
    fn safe_rel_blocks_escape() {
        assert!(safe_rel("scripts/run.py"));
        assert!(!safe_rel("../x"));
        assert!(!safe_rel("/etc/passwd"));
        assert!(!safe_rel("C:/x"));
        assert!(!safe_rel("a//b"));
    }

    #[test]
    fn fingerprint_changes_on_new_file() {
        let t = TestDir::new("fp");
        t.write("a/SKILL.md", "x");
        let f1 = fingerprint(&[t.path().to_path_buf()]);
        t.write("b/SKILL.md", "y");
        assert_ne!(f1, fingerprint(&[t.path().to_path_buf()]));
    }
}
