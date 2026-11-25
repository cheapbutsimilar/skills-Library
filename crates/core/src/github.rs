//! GitHub 저장소·하위 폴더 URL에서 스킬 가져오기.
//!
//! - URL 읽기: `https://github.com/owner/repo`, `…/tree/<branch>/<path>`, `…/blob/<branch>/<path>/SKILL.md`,
//!   `github.com/owner/repo.git`, `owner/repo` 모두 받아요.
//! - 내려받기는 앱(src-tauri)이 해요. 여기서는 `codeload.github.com` ZIP 주소를 만들고,
//!   받은 ZIP에서 고른 폴더만 풀어서 스킬 폴더를 찾고 검사해요 (네트워크·OS 의존 없음).
//! - 브랜치 이름에 `/`가 있으면 URL만으로는 브랜치와 경로를 나눌 수 없어서, 짧은 것부터 차례로 시도해요.

use crate::fsutil;
use crate::library::read_skill_dir;
use crate::validate;
use crate::zipio::find_skill_roots;
use serde::Serialize;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const MAX_TOTAL: u64 = 200 * 1024 * 1024;
const MAX_ENTRIES: usize = 20000;
/// 저장소 맨 위에서 이만큼 아래까지 스킬 폴더를 찾아요 (`skills/<name>/SKILL.md`, `plugins/x/skills/<name>` …).
const FIND_DEPTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GhUrl {
    pub owner: String,
    pub repo: String,
    /// `tree/`·`blob/` 뒤의 조각들 (브랜치 + 경로). 비어 있으면 기본 브랜치의 맨 위예요.
    pub rest: Vec<String>,
    /// `blob/…` 주소면 파일을 가리키니 그 파일이 든 폴더를 가져와요.
    pub blob: bool,
}

/// 내려받기 시도 하나: 이 브랜치(또는 태그·커밋)의 ZIP에서 이 경로를 가져와요.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub reference: String,
    pub subpath: String,
}

fn valid_part(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) && s != "." && s != ".."
}

/// URL을 읽어요. GitHub 주소가 아니면 이유를 알려줘요.
pub fn parse_url(input: &str) -> Result<GhUrl, String> {
    let mut s = input.trim().to_string();
    if s.is_empty() {
        return Err(tr!("GitHub 주소를 붙여 넣어 주세요", "Paste a GitHub URL"));
    }
    if let Some(i) = s.find(['?', '#']) {
        s.truncate(i);
    }
    let lower = s.to_ascii_lowercase();
    let rest = if let Some(r) = ["https://", "http://"].iter().find_map(|p| lower.strip_prefix(p).map(|_| &s[p.len()..])) {
        let r = r.strip_prefix("www.").unwrap_or(r);
        if !r.to_ascii_lowercase().starts_with("github.com/") {
            return Err(tr!("github.com 주소만 가져올 수 있어요", "Only github.com URLs are supported"));
        }
        r["github.com/".len()..].to_string()
    } else if lower.starts_with("github.com/") || lower.starts_with("www.github.com/") {
        s[s.to_ascii_lowercase().find("github.com/").unwrap() + "github.com/".len()..].to_string()
    } else if lower.starts_with("git@github.com:") {
        s["git@github.com:".len()..].to_string()
    } else if !s.contains("://") && s.matches('/').count() >= 1 && !s.starts_with('/') {
        s.clone()
    } else {
        return Err(tr!("github.com 주소만 가져올 수 있어요", "Only github.com URLs are supported"));
    };
    let parts: Vec<String> = rest.split('/').filter(|p| !p.is_empty()).map(|p| p.to_string()).collect();
    if parts.len() < 2 {
        return Err(tr!(
            "저장소 주소가 아니에요 — https://github.com/owner/repo 모양이어야 해요",
            "That is not a repository URL — it should look like https://github.com/owner/repo"
        ));
    }
    let owner = parts[0].clone();
    let repo = parts[1].trim_end_matches(".git").to_string();
    if !valid_part(&owner) || !valid_part(&repo) {
        return Err(tr!("저장소 이름이 올바르지 않아요", "The repository name is not valid"));
    }
    let mut blob = false;
    let rest: Vec<String> = match parts.get(2).map(|s| s.as_str()) {
        None => vec![],
        Some("tree") | Some("blob") => {
            blob = parts[2] == "blob";
            parts[3..].iter().map(|p| percent_decode(p)).collect()
        }
        Some(_) => {
            return Err(tr!(
                "저장소나 폴더 주소가 아니에요 — 폴더는 …/tree/<브랜치>/<경로> 모양이에요",
                "Not a repository or folder URL — folders look like …/tree/<branch>/<path>"
            ))
        }
    };
    if rest.iter().any(|p| p == ".." || p == "." || p.contains('\\')) {
        return Err(tr!("경로가 올바르지 않아요", "The path is not valid"));
    }
    if parts.len() > 2 && rest.is_empty() {
        return Err(tr!("브랜치 이름이 빠졌어요", "The branch name is missing"));
    }
    Ok(GhUrl { owner, repo, rest, blob })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

impl GhUrl {
    /// 시도할 (브랜치, 경로) 목록. 브랜치가 없으면 기본 브랜치(`HEAD`) 하나예요.
    pub fn candidates(&self) -> Vec<Candidate> {
        if self.rest.is_empty() {
            return vec![Candidate { reference: "HEAD".into(), subpath: String::new() }];
        }
        let mut out = Vec::new();
        for k in 1..=self.rest.len().min(6) {
            let reference = self.rest[..k].join("/");
            let mut path: Vec<&str> = self.rest[k..].iter().map(|s| s.as_str()).collect();
            if self.blob {
                // 파일 주소면 그 파일이 든 폴더예요. 파일 이름이 없으면(브랜치만) 이 후보는 버려요.
                if path.is_empty() {
                    continue;
                }
                path.pop();
            }
            out.push(Candidate { reference, subpath: path.join("/") });
        }
        out
    }

    pub fn zip_url(&self, reference: &str) -> String {
        let r: String = reference.split('/').map(encode_part).collect::<Vec<_>>().join("/");
        format!("https://codeload.github.com/{}/{}/zip/{r}", self.owner, self.repo)
    }

    /// 화면에 보여줄 짧은 이름: `owner/repo` 또는 `owner/repo/path`
    pub fn label(&self) -> String {
        let mut s = format!("{}/{}", self.owner, self.repo);
        if self.rest.len() > 1 {
            s.push('/');
            s.push_str(&self.rest[1..].join("/"));
        }
        s
    }
}

fn encode_part(p: &str) -> String {
    p.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// codeload ZIP(맨 위에 `repo-ref/` 폴더 하나)에서 `subpath` 아래만 `dest/<이름>`에 풀어요.
/// 하위 폴더가 ZIP에 없으면 `Ok(None)` — 다음 후보(브랜치 이름에 `/`가 있는 경우)를 시도해요.
pub fn extract_subdir(zip_bytes: &[u8], subpath: &str, dest: &Path, repo: &str) -> Result<Option<PathBuf>, String> {
    let mut za = zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|_| tr!("받은 파일이 ZIP이 아니에요", "The download is not a ZIP file"))?;
    if za.len() > MAX_ENTRIES {
        return Err(tr!("저장소에 파일이 너무 많아요 — 스킬 폴더 주소를 붙여 넣어 주세요", "The repository has too many files — paste the skill folder URL instead"));
    }
    let sub: Vec<&str> = subpath.split('/').filter(|p| !p.is_empty()).collect();
    let name = sub.last().copied().unwrap_or(repo);
    let root = dest.join(if valid_part(name) { name } else { "skill" });
    let mut total = 0u64;
    let mut found = false;
    for i in 0..za.len() {
        let mut entry = za.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let comps: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
        // 맨 위 `repo-ref/`를 떼고, 고른 경로 아래만 남겨요.
        if comps.len() <= 1 || comps.len() - 1 < sub.len() || comps[1..=sub.len()].iter().zip(&sub).any(|(a, b)| a != b) {
            continue;
        }
        let inner = &comps[1 + sub.len()..];
        found = true;
        if inner.is_empty() || inner.iter().any(|s| fsutil::is_ignored(s) || s == "__MACOSX") {
            continue;
        }
        let out = inner.iter().fold(root.clone(), |p, c| p.join(c));
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        total += entry.size();
        if total > MAX_TOTAL {
            return Err(tr!("내용이 너무 커요 (200MB까지)", "The content is too large (200 MB max)"));
        }
        if let Some(p) = out.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut buf = Vec::with_capacity(entry.size().min(MAX_TOTAL) as usize);
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        fs::write(&out, buf).map_err(|e| e.to_string())?;
    }
    if !found {
        return Ok(None);
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    Ok(Some(root))
}

/// 푼 폴더에서 스킬 폴더를 찾고 검사해요. (가져올 폴더, 건너뛴 이유)
/// 오류(에이전트가 못 읽는 것)가 있는 스킬은 가져오지 않아요. 경고는 괜찮아요.
pub fn validated_roots(dir: &Path) -> (Vec<PathBuf>, Vec<String>) {
    let mut ok = Vec::new();
    let mut skipped = Vec::new();
    for r in find_skill_roots(dir, FIND_DEPTH) {
        let (_, issues, _) = read_skill_dir(&r);
        let name = r.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match issues.iter().find(|i| i.level == validate::Level::Error) {
            Some(e) => skipped.push(format!("{name}: {}", e.msg)),
            None => ok.push(r),
        }
    }
    (ok, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;
    use std::io::Write;

    fn c(r: &str, p: &str) -> Candidate {
        Candidate { reference: r.into(), subpath: p.into() }
    }

    #[test]
    fn parses_repo_urls() {
        let u = parse_url("https://github.com/anthropics/skills").unwrap();
        assert_eq!((u.owner.as_str(), u.repo.as_str()), ("anthropics", "skills"));
        assert_eq!(u.candidates(), vec![c("HEAD", "")]);
        assert_eq!(u.zip_url("HEAD"), "https://codeload.github.com/anthropics/skills/zip/HEAD");
        for s in ["github.com/anthropics/skills.git", "https://www.github.com/anthropics/skills/", "anthropics/skills", "git@github.com:anthropics/skills.git", " https://github.com/anthropics/skills?tab=readme#x "] {
            let u = parse_url(s).unwrap();
            assert_eq!((u.owner.as_str(), u.repo.as_str(), u.rest.len()), ("anthropics", "skills", 0), "{s}");
        }
    }

    #[test]
    fn parses_tree_and_blob_urls() {
        let u = parse_url("https://github.com/anthropics/skills/tree/main/skills/pdf").unwrap();
        assert_eq!(u.candidates()[0], c("main", "skills/pdf"));
        assert_eq!(u.candidates()[1], c("main/skills", "pdf"));
        assert_eq!(u.label(), "anthropics/skills/skills/pdf");
        let u = parse_url("https://github.com/o/r/blob/dev/a/b/SKILL.md").unwrap();
        assert_eq!(u.candidates(), vec![c("dev", "a/b"), c("dev/a", "b"), c("dev/a/b", "")]);
        let u = parse_url("https://github.com/o/r/tree/feature%2Fx").unwrap();
        assert_eq!(u.candidates(), vec![c("feature/x", "")]);
        assert_eq!(u.zip_url("feature/x"), "https://codeload.github.com/o/r/zip/feature/x");
    }

    #[test]
    fn rejects_other_urls() {
        assert!(parse_url("").is_err());
        assert!(parse_url("https://gitlab.com/a/b").is_err());
        assert!(parse_url("https://github.com/only-owner").is_err());
        assert!(parse_url("https://github.com/o/r/issues/3").is_err());
        assert!(parse_url("https://github.com/o/r/tree/").is_err());
        assert!(parse_url("https://github.com/o/r/tree/main/../x").is_err());
        assert!(parse_url("https://github.com/o/r$/").is_err());
    }

    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            for (p, body) in files {
                zw.start_file(*p, o).unwrap();
                zw.write_all(body.as_bytes()).unwrap();
            }
            zw.finish().unwrap();
        }
        buf.into_inner()
    }

    const OK: &str = "---\nname: pdf\ndescription: Work with PDF files when the user asks to merge or split them\n---\n\n# PDF\n";

    #[test]
    fn extracts_only_the_subfolder() {
        let t = TestDir::new("gh-sub");
        let z = zip_of(&[
            ("skills-main/README.md", "x"),
            ("skills-main/skills/pdf/SKILL.md", OK),
            ("skills-main/skills/pdf/scripts/run.py", "print(1)"),
            ("skills-main/skills/other/SKILL.md", "---\nname: other\ndescription: d\n---\n"),
        ]);
        let root = extract_subdir(&z, "skills/pdf", t.path(), "skills").unwrap().unwrap();
        assert_eq!(root, t.path().join("pdf"));
        assert!(root.join("SKILL.md").is_file() && root.join("scripts/run.py").is_file());
        assert!(!t.path().join("other").exists());
        let (ok, skipped) = validated_roots(t.path());
        assert_eq!(ok, vec![root]);
        assert!(skipped.is_empty());
        // 없는 경로는 None → 다음 후보로
        assert_eq!(extract_subdir(&z, "nope", &t.path().join("n"), "skills").unwrap(), None);
    }

    #[test]
    fn whole_repo_finds_many_and_skips_invalid() {
        let t = TestDir::new("gh-all");
        let z = zip_of(&[
            ("skills-HEAD/skills/pdf/SKILL.md", OK),
            ("skills-HEAD/skills/broken/SKILL.md", "no frontmatter"),
            ("skills-HEAD/.github/x/SKILL.md", OK),
            ("skills-HEAD/a/b/c/deep/SKILL.md", &OK.replace("name: pdf", "name: deep")),
        ]);
        let root = extract_subdir(&z, "", t.path(), "skills").unwrap().unwrap();
        assert_eq!(root, t.path().join("skills"));
        let (ok, skipped) = validated_roots(&root);
        let names: Vec<_> = ok.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, vec!["deep", "pdf"]);
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].starts_with("broken: "));
    }

    #[test]
    fn rejects_non_zip_download() {
        let t = TestDir::new("gh-bad");
        assert!(extract_subdir(b"<html>404</html>", "", t.path(), "r").is_err());
    }
}
