//! ZIP 내보내기·가져오기. Claude 앱(claude.ai·데스크톱)에는 스킬 폴더가 통째로 든 ZIP을 올려요.

use crate::fsutil::{self, join_rel};
use crate::library::has_skill_md;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const MAX_TOTAL: u64 = 200 * 1024 * 1024;
const MAX_ENTRIES: usize = 5000;

/// `<slug>/…` 구조의 ZIP을 만들어요.
pub fn export(skill_dir: &Path, slug: &str, out: &Path) -> io::Result<()> {
    if let Some(p) = out.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = out.with_extension("zip.tmp");
    {
        let f = fs::File::create(&tmp)?;
        let mut zw = zip::ZipWriter::new(f);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);
        for rel in fsutil::list_files(skill_dir)? {
            zw.start_file(format!("{slug}/{rel}"), opts).map_err(io::Error::other)?;
            let bytes = fs::read(join_rel(skill_dir, &rel))?;
            zw.write_all(&bytes)?;
        }
        zw.finish().map_err(io::Error::other)?;
    }
    fs::rename(&tmp, out)
}

/// ZIP(또는 .skill)을 `dest`에 풀고, 그 안의 스킬 폴더들을 돌려줘요.
/// 폴더 밖으로 나가는 경로(zip slip)는 막아요.
pub fn extract(zip_path: &Path, dest: &Path) -> Result<Vec<PathBuf>, String> {
    let f = fs::File::open(zip_path).map_err(|e| format!("파일을 열지 못했어요 — {e}"))?;
    let mut za = zip::ZipArchive::new(f).map_err(|_| "ZIP 파일이 아니거나 깨졌어요".to_string())?;
    if za.len() > MAX_ENTRIES {
        return Err("ZIP 안에 파일이 너무 많아요".into());
    }
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut total = 0u64;
    for i in 0..za.len() {
        let mut entry = za.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else { continue };
        if rel.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == "__MACOSX" || fsutil::is_ignored(&s)
        }) {
            continue;
        }
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        total += entry.size();
        if total > MAX_TOTAL {
            return Err("ZIP 내용이 너무 커요 (200MB까지)".into());
        }
        if let Some(p) = out.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        fs::write(&out, buf).map_err(|e| e.to_string())?;
    }
    let roots = find_skill_roots(dest, 3);
    if roots.is_empty() {
        return Err("ZIP 안에서 SKILL.md를 찾지 못했어요".into());
    }
    Ok(roots)
}

/// SKILL.md가 있는 폴더를 찾아요. 스킬 폴더 안쪽은 더 내려가지 않아요.
pub fn find_skill_roots(dir: &Path, depth: usize) -> Vec<PathBuf> {
    if has_skill_md(dir) {
        return vec![dir.to_path_buf()];
    }
    let mut out = Vec::new();
    if depth == 0 {
        return out;
    }
    if let Ok(rd) = fs::read_dir(dir) {
        let mut dirs: Vec<_> = rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.path()).collect();
        dirs.sort();
        for d in dirs {
            let name = d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name.starts_with('.') || fsutil::is_ignored(&name) {
                continue;
            }
            out.extend(find_skill_roots(&d, depth - 1));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;

    #[test]
    fn roundtrip() {
        let t = TestDir::new("zip");
        t.write("s/SKILL.md", "---\nname: s\ndescription: d\n---\n");
        t.write("s/scripts/run.py", "print(1)");
        let z = t.path().join("out/s.zip");
        export(&t.path().join("s"), "s", &z).unwrap();
        let roots = extract(&z, &t.path().join("x")).unwrap();
        assert_eq!(roots, vec![t.path().join("x/s")]);
        assert_eq!(fsutil::hash_dir(&roots[0]).unwrap(), fsutil::hash_dir(&t.path().join("s")).unwrap());
    }

    #[test]
    fn finds_multiple_skills() {
        let t = TestDir::new("roots");
        t.write("pack/a/SKILL.md", "x");
        t.write("pack/b/SKILL.md", "x");
        t.write("pack/b/inner/SKILL.md", "x");
        let r = find_skill_roots(&t.path().join("pack"), 3);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn rejects_non_zip() {
        let t = TestDir::new("nz");
        let p = t.write("x.zip", "not a zip");
        assert!(extract(&p, &t.path().join("o")).is_err());
    }
}
