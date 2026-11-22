//! 라이브러리 폴더: `<library>/<slug>/SKILL.md` 모양의 스킬 묶음이에요.
//! 태그·즐겨찾기 같은 부가 정보는 `<library>/.skills-library.json`에 둬서 폴더째 옮겨도 따라가요.

use crate::atomic;
use crate::fsutil::{self, join_rel, safe_rel};
use crate::skill::{self, SKILL_FILE};
use crate::validate::{self, Issue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const META_FILE: &str = ".skills-library.json";
/// 텍스트로 열어 편집할 수 있는 파일 크기 상한.
pub const TEXT_MAX: u64 = 512 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SkillMeta {
    pub tags: Vec<String>,
    pub favorite: bool,
    pub created: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibraryMeta {
    pub v: u32,
    pub skills: BTreeMap<String, SkillMeta>,
}

impl Default for LibraryMeta {
    fn default() -> Self {
        LibraryMeta { v: 1, skills: BTreeMap::new() }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibSkill {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub hash: String,
    pub files: usize,
    pub bytes: u64,
    pub updated: u64,
    pub issues: Vec<Issue>,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub bytes: u64,
    pub text: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub slug: String,
    pub dir: String,
    pub raw: String,
    pub name: String,
    pub description: String,
    pub body: String,
    pub files: Vec<FileEntry>,
    pub issues: Vec<Issue>,
}

/// 스킬 폴더 하나를 읽어서 요약해요 (라이브러리·설치 위치 공통).
pub fn read_skill_dir(dir: &Path) -> (skill::Parsed, Vec<Issue>, bool) {
    let path = dir.join(SKILL_FILE);
    let (raw, has) = match fs::read_to_string(&path) {
        Ok(s) => (s, true),
        Err(_) => match fs::read_to_string(dir.join("skill.md")) {
            Ok(s) => (s, true),
            Err(_) => (String::new(), false),
        },
    };
    let p = skill::parse(&raw);
    let dir_name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let issues = validate::check(&p, &dir_name, has);
    (p, issues, has)
}

pub fn has_skill_md(dir: &Path) -> bool {
    dir.join(SKILL_FILE).is_file() || dir.join("skill.md").is_file()
}

fn looks_text(bytes: &[u8]) -> bool {
    !bytes.iter().take(8000).any(|b| *b == 0) && std::str::from_utf8(bytes).is_ok()
}

pub struct Library {
    pub dir: PathBuf,
}

impl Library {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Library { dir: dir.into() }
    }

    pub fn path(&self, slug: &str) -> PathBuf {
        self.dir.join(slug)
    }

    pub fn exists(&self, slug: &str) -> bool {
        self.path(slug).is_dir()
    }

    fn check_slug(slug: &str) -> Result<(), String> {
        if slug.is_empty() || slug.starts_with('.') || !safe_rel(slug) || slug.contains('/') || slug.contains('\\') {
            return Err("스킬 이름이 올바르지 않아요".into());
        }
        Ok(())
    }

    pub fn meta(&self) -> LibraryMeta {
        atomic::read_json(&self.dir.join(META_FILE)).unwrap_or_default()
    }

    pub fn save_meta(&self, meta: &LibraryMeta) -> Result<(), String> {
        atomic::write_json(&self.dir.join(META_FILE), meta).map_err(|e| format!("라이브러리 정보를 저장하지 못했어요 — {e}"))
    }

    fn update_meta(&self, f: impl FnOnce(&mut LibraryMeta)) -> Result<(), String> {
        let mut m = self.meta();
        f(&mut m);
        self.save_meta(&m)
    }

    /// 라이브러리 안의 스킬 폴더 이름 목록 (점으로 시작하는 폴더는 빼요).
    pub fn slugs(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || fsutil::is_ignored(&name) {
                    continue;
                }
                if e.path().is_dir() {
                    out.push(name);
                }
            }
        }
        out.sort();
        out
    }

    pub fn list(&self) -> Vec<LibSkill> {
        let meta = self.meta();
        self.slugs()
            .into_iter()
            .map(|slug| {
                let dir = self.path(&slug);
                let (p, issues, _) = read_skill_dir(&dir);
                let st = fsutil::dir_stats(&dir);
                let m = meta.skills.get(&slug).cloned().unwrap_or_default();
                LibSkill {
                    name: if p.name.is_empty() { slug.clone() } else { p.name },
                    description: p.description,
                    hash: fsutil::hash_dir(&dir).unwrap_or_default(),
                    files: st.files,
                    bytes: st.bytes,
                    updated: st.updated,
                    issues,
                    tags: m.tags,
                    favorite: m.favorite,
                    slug,
                }
            })
            .collect()
    }

    pub fn detail(&self, slug: &str) -> Result<SkillDetail, String> {
        Self::check_slug(slug)?;
        let dir = self.path(slug);
        if !dir.is_dir() {
            return Err("라이브러리에 그 스킬이 없어요".into());
        }
        let raw = fs::read_to_string(dir.join(SKILL_FILE)).unwrap_or_default();
        let (p, issues, _) = read_skill_dir(&dir);
        let files = fsutil::list_files(&dir)
            .unwrap_or_default()
            .into_iter()
            .map(|rel| {
                let full = join_rel(&dir, &rel);
                let bytes = fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
                let text = bytes <= TEXT_MAX && fs::read(&full).map(|b| looks_text(&b)).unwrap_or(false);
                FileEntry { path: rel, bytes, text }
            })
            .collect();
        Ok(SkillDetail {
            slug: slug.into(),
            dir: dir.to_string_lossy().into(),
            raw,
            name: p.name,
            description: p.description,
            body: p.body,
            files,
            issues,
        })
    }

    pub fn unique_slug(&self, base: &str) -> String {
        let base = skill::slugify(base);
        if !self.exists(&base) {
            return base;
        }
        for i in 2..1000 {
            let suffix = format!("-{i}");
            let stem: String = base.chars().take(64 - suffix.len()).collect();
            let cand = format!("{}{}", stem.trim_end_matches('-'), suffix);
            if !self.exists(&cand) {
                return cand;
            }
        }
        format!("{}-{}", base.chars().take(48).collect::<String>(), fsutil::now_ms())
    }

    pub fn create(&self, name: &str, description: &str) -> Result<String, String> {
        let name = name.trim();
        if !skill::is_valid_name(name) {
            return Err("이름은 영어 소문자·숫자·하이픈(-)으로 64자까지 쓸 수 있어요".into());
        }
        if self.exists(name) {
            return Err(format!("'{name}' 스킬이 이미 있어요"));
        }
        let dir = self.path(name);
        fs::create_dir_all(&dir).map_err(|e| format!("폴더를 만들지 못했어요 — {e}"))?;
        let raw = skill::compose(name, description.trim(), &skill::template_body(name));
        fs::write(dir.join(SKILL_FILE), raw).map_err(|e| format!("SKILL.md를 쓰지 못했어요 — {e}"))?;
        let now = fsutil::now_ms();
        self.update_meta(|m| {
            m.skills.insert(name.into(), SkillMeta { created: now, ..Default::default() });
        })?;
        Ok(name.into())
    }

    /// SKILL.md를 저장해요. name이 바뀌었고 올바르면 폴더 이름도 맞춰요. (새 slug, 바뀌기 전 slug)를 돌려줘요.
    pub fn save_raw(&self, slug: &str, raw: &str) -> Result<(String, Option<String>), String> {
        Self::check_slug(slug)?;
        let dir = self.path(slug);
        if !dir.is_dir() {
            return Err("라이브러리에 그 스킬이 없어요".into());
        }
        atomic::write_atomic(&dir.join(SKILL_FILE), raw.as_bytes()).map_err(|e| format!("저장하지 못했어요 — {e}"))?;
        let _ = fs::remove_file(atomic::backup_path(&dir.join(SKILL_FILE)));
        let p = skill::parse(raw);
        if p.name != slug && skill::is_valid_name(&p.name) && !self.exists(&p.name) {
            self.rename(slug, &p.name)?;
            return Ok((p.name, Some(slug.into())));
        }
        Ok((slug.into(), None))
    }

    pub fn rename(&self, old: &str, new: &str) -> Result<(), String> {
        Self::check_slug(old)?;
        Self::check_slug(new)?;
        fs::rename(self.path(old), self.path(new)).map_err(|e| format!("폴더 이름을 바꾸지 못했어요 — {e}"))?;
        self.update_meta(|m| {
            if let Some(v) = m.skills.remove(old) {
                m.skills.insert(new.into(), v);
            }
        })
    }

    pub fn duplicate(&self, slug: &str) -> Result<String, String> {
        Self::check_slug(slug)?;
        let new = self.unique_slug(&format!("{slug}-copy"));
        fsutil::copy_tree(&self.path(slug), &self.path(&new)).map_err(|e| format!("복제하지 못했어요 — {e}"))?;
        let raw = fs::read_to_string(self.path(&new).join(SKILL_FILE)).unwrap_or_default();
        let p = skill::parse(&raw);
        if let Ok(out) = skill::with_fields(&raw, &new, &p.description, None) {
            let _ = fs::write(self.path(&new).join(SKILL_FILE), out);
        }
        let mut meta = self.meta();
        let mut m = meta.skills.get(slug).cloned().unwrap_or_default();
        m.created = fsutil::now_ms();
        m.favorite = false;
        meta.skills.insert(new.clone(), m);
        self.save_meta(&meta)?;
        Ok(new)
    }

    pub fn delete(&self, slug: &str, trash: &Path) -> Result<(), String> {
        Self::check_slug(slug)?;
        fsutil::move_to_trash(&self.path(slug), trash, &format!("library-{slug}"))
            .map_err(|e| format!("지우지 못했어요 — {e}"))?;
        self.update_meta(|m| {
            m.skills.remove(slug);
        })
    }

    /// 다른 폴더의 스킬을 라이브러리로 복사해요. name이 올바르면 그걸 폴더 이름으로 써요.
    /// `preferred`가 주어지면(설치 위치에서 가져올 때) 그 이름을 먼저 써요.
    pub fn import_dir(&self, src: &Path, preferred: Option<&str>) -> Result<String, String> {
        if !has_skill_md(src) {
            return Err("SKILL.md가 있는 폴더가 아니에요".into());
        }
        let (p, _, _) = read_skill_dir(src);
        let dir_name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let base = preferred
            .map(|s| s.to_string())
            .or_else(|| skill::is_valid_name(&p.name).then(|| p.name.clone()))
            .unwrap_or(dir_name);
        let slug = self.unique_slug(&base);
        fs::create_dir_all(&self.dir).map_err(|e| format!("라이브러리 폴더를 만들지 못했어요 — {e}"))?;
        fsutil::replace_dir(src, &self.path(&slug)).map_err(|e| format!("가져오지 못했어요 — {e}"))?;
        // 예전 이름 규칙(skill.md)이면 SKILL.md로 맞춰요.
        let lower = self.path(&slug).join("skill.md");
        if lower.is_file() && !self.path(&slug).join(SKILL_FILE).is_file() {
            let _ = fs::rename(&lower, self.path(&slug).join(SKILL_FILE));
        }
        let now = fsutil::now_ms();
        self.update_meta(|m| {
            m.skills.entry(slug.clone()).or_insert(SkillMeta { created: now, ..Default::default() });
        })?;
        Ok(slug)
    }

    /// 라이브러리 스킬을 다른 곳의 내용으로 바꿔요 (설치 위치에서 "가져오기"). 이전 내용은 휴지통으로.
    pub fn replace_from(&self, slug: &str, src: &Path, trash: &Path) -> Result<(), String> {
        Self::check_slug(slug)?;
        let dst = self.path(slug);
        if dst.is_dir() {
            let bak = trash.join(format!("{}-library-{slug}-before-pull", fsutil::now_ms()));
            fsutil::copy_tree(&dst, &bak.join(slug)).map_err(|e| format!("백업하지 못했어요 — {e}"))?;
        }
        fsutil::replace_dir(src, &dst).map_err(|e| format!("가져오지 못했어요 — {e}"))
    }

    pub fn read_file(&self, slug: &str, rel: &str) -> Result<String, String> {
        Self::check_slug(slug)?;
        if !safe_rel(rel) {
            return Err("파일 경로가 올바르지 않아요".into());
        }
        let p = join_rel(&self.path(slug), rel);
        let bytes = fs::read(&p).map_err(|e| format!("파일을 읽지 못했어요 — {e}"))?;
        if bytes.len() as u64 > TEXT_MAX || !looks_text(&bytes) {
            return Err("텍스트 파일만 여기서 열 수 있어요".into());
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn write_file(&self, slug: &str, rel: &str, content: &str) -> Result<(), String> {
        Self::check_slug(slug)?;
        if !safe_rel(rel) || rel.split('/').any(|p| p.starts_with('.')) {
            return Err("파일 경로가 올바르지 않아요 (점으로 시작하는 이름·'..'은 쓸 수 없어요)".into());
        }
        let p = join_rel(&self.path(slug), rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("폴더를 만들지 못했어요 — {e}"))?;
        }
        atomic::write_atomic(&p, content.as_bytes()).map_err(|e| format!("저장하지 못했어요 — {e}"))?;
        let _ = fs::remove_file(atomic::backup_path(&p));
        Ok(())
    }

    pub fn delete_file(&self, slug: &str, rel: &str, trash: &Path) -> Result<(), String> {
        Self::check_slug(slug)?;
        if !safe_rel(rel) {
            return Err("파일 경로가 올바르지 않아요".into());
        }
        if rel == SKILL_FILE {
            return Err("SKILL.md는 지울 수 없어요".into());
        }
        let p = join_rel(&self.path(slug), rel);
        fsutil::move_to_trash(&p, trash, &format!("file-{slug}")).map_err(|e| format!("지우지 못했어요 — {e}"))?;
        Ok(())
    }

    pub fn set_meta(&self, slug: &str, tags: Option<Vec<String>>, favorite: Option<bool>) -> Result<(), String> {
        Self::check_slug(slug)?;
        self.update_meta(|m| {
            let e = m.skills.entry(slug.into()).or_default();
            if let Some(tags) = tags {
                let mut clean: Vec<String> = tags
                    .into_iter()
                    .map(|t| t.trim().chars().take(24).collect::<String>())
                    .filter(|t| !t.is_empty())
                    .collect();
                clean.sort();
                clean.dedup();
                clean.truncate(12);
                e.tags = clean;
            }
            if let Some(f) = favorite {
                e.favorite = f;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;

    #[test]
    fn create_list_save_rename() {
        let t = TestDir::new("lib");
        let lib = Library::new(t.path().join("lib"));
        assert!(lib.create("Bad Name", "x").is_err());
        let slug = lib.create("hello", "인사하는 스킬이에요. 사용자가 인사하면 써요.").unwrap();
        assert_eq!(slug, "hello");
        assert!(lib.create("hello", "dup").is_err());
        let list = lib.list();
        assert_eq!(list.len(), 1);
        assert!(list[0].issues.is_empty(), "{:?}", list[0].issues);

        let d = lib.detail("hello").unwrap();
        let raw = skill::with_fields(&d.raw, "hi-there", &d.description, None).unwrap();
        let (new, old) = lib.save_raw("hello", &raw).unwrap();
        assert_eq!(new, "hi-there");
        assert_eq!(old.as_deref(), Some("hello"));
        assert!(lib.exists("hi-there") && !lib.exists("hello"));
        assert!(lib.meta().skills.contains_key("hi-there"));
    }

    #[test]
    fn import_uses_frontmatter_name_and_unique_suffix() {
        let t = TestDir::new("imp");
        t.write("src/whatever/SKILL.md", "---\nname: pdf\ndescription: d\n---\nbody");
        let lib = Library::new(t.path().join("lib"));
        assert_eq!(lib.import_dir(&t.path().join("src/whatever"), None).unwrap(), "pdf");
        assert_eq!(lib.import_dir(&t.path().join("src/whatever"), None).unwrap(), "pdf-2");
        assert!(lib.import_dir(t.path(), None).is_err());
    }

    #[test]
    fn files_are_sandboxed() {
        let t = TestDir::new("files");
        let lib = Library::new(t.path().join("lib"));
        lib.create("a", "desc").unwrap();
        lib.write_file("a", "ref/notes.md", "hi").unwrap();
        assert_eq!(lib.read_file("a", "ref/notes.md").unwrap(), "hi");
        assert!(lib.write_file("a", "../evil.md", "x").is_err());
        assert!(lib.write_file("a", ".hidden", "x").is_err());
        assert!(lib.delete_file("a", "SKILL.md", &t.path().join("trash")).is_err());
        lib.delete_file("a", "ref/notes.md", &t.path().join("trash")).unwrap();
        assert!(lib.read_file("a", "ref/notes.md").is_err());
    }

    #[test]
    fn duplicate_renames_frontmatter() {
        let t = TestDir::new("dup");
        let lib = Library::new(t.path().join("lib"));
        lib.create("tool", "설명").unwrap();
        let d = lib.duplicate("tool").unwrap();
        assert_eq!(d, "tool-copy");
        assert_eq!(lib.detail(&d).unwrap().name, "tool-copy");
    }

    #[test]
    fn meta_tags_are_cleaned() {
        let t = TestDir::new("meta");
        let lib = Library::new(t.path().join("lib"));
        lib.create("a", "d").unwrap();
        lib.set_meta("a", Some(vec![" pdf ".into(), "pdf".into(), "".into(), "문서".into()]), Some(true)).unwrap();
        let m = lib.meta().skills["a"].clone();
        assert_eq!(m.tags, vec!["pdf".to_string(), "문서".to_string()]);
        assert!(m.favorite);
    }
}
