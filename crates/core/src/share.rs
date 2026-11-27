//! 공유 번들: 7자리 코드·링크 읽기와 라이브러리 묶음(ZIP) 만들기.
//!
//! - 코드는 7자리 영문 대문자·숫자예요 (`AB3K9MZ`). 서버가 혼동하기 쉬운 `0 O 1 I L`을 빼고 만들어요.
//! - 링크는 `https://skills-library-share.sakugarchive.workers.dev/bundles/AB3K9MZ` 모양이고,
//!   앞의 주소는 버리고 끝의 코드만 써요. 그래서 어느 주소로 받은 링크든 같은 코드로 풀려요.
//! - 올리고 내려받는 일(HTTP)은 앱(src-tauri)이 해요. 여기서는 ZIP을 묶고, 받은 글자에서 코드를 뽑고, 크기를 재요.
//! - 묶음은 `<스킬 이름>/…` 구조예요. 앱의 ZIP 가져오기와 같은 모양이라 검사·중복 확인 흐름을 그대로 써요.

use crate::fsutil::{self, join_rel};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 공유 API 주소 (Cloudflare Workers + R2).
pub const BASE_URL: &str = "https://skills-library-share.sakugarchive.workers.dev";
/// 서버가 정하는 코드 길이.
pub const CODE_LEN: usize = 7;
/// 서버가 받는 최대 크기. 넘으면 413으로 거절해요.
pub const MAX_BYTES: u64 = 2 * 1024 * 1024;
/// 서버가 정한 만료 기간(일). 화면 안내에도 같은 값을 써요.
pub const EXPIRES_IN_DAYS: u32 = 30;

/// 코드로 만든 공유 링크.
pub fn share_url(code: &str) -> String {
    format!("{BASE_URL}/bundles/{code}")
}

fn is_code(s: &str) -> bool {
    s.len() == CODE_LEN && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// 7자리 코드 또는 공유 링크에서 코드만 뽑아요.
/// 앞뒤 공백, 소문자, 끝의 `/`, `?…`·`#…`는 받아 줘요.
pub fn parse_code(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(tr!("공유 코드나 링크를 붙여 넣어 주세요", "Paste a share code or link"));
    }
    let cut = raw.find(['?', '#']).map(|i| &raw[..i]).unwrap_or(raw);
    let code = cut.trim().trim_end_matches('/').rsplit('/').next().unwrap_or("").to_ascii_uppercase();
    if !is_code(&code) {
        return Err(tr!(
            "공유 코드가 아니에요 — 7자리 코드(예: AB3K9MZ)나 공유 링크를 붙여 넣어 주세요",
            "That is not a share code — paste a 7-character code (like AB3K9MZ) or a share link"
        ));
    }
    Ok(code)
}

/// 사람이 쓴 글자에서 코드처럼 보이는지 (화면에서 GitHub 주소와 나눠 부를 때 써요).
pub fn looks_like_code(input: &str) -> bool {
    parse_code(input).is_ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    /// ZIP 파일 크기(바이트)
    pub bytes: u64,
    /// 담은 스킬 수
    pub count: usize,
}

/// 고른 스킬 폴더들을 `<스킬 이름>/…` 구조의 ZIP 하나로 묶어요.
/// 서버 한도(2MB)를 넘으면 파일을 지우고 안내와 함께 실패해요.
pub fn pack(skills: &[(String, PathBuf)], out: &Path) -> Result<Bundle, String> {
    if skills.is_empty() {
        return Err(tr!("공유할 스킬을 골라 주세요", "Pick at least one skill to share"));
    }
    if let Some(p) = out.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let tmp = out.with_extension("zip.tmp");
    let mut count = 0;
    {
        let f = fs::File::create(&tmp).map_err(|e| tr!("ZIP을 만들지 못했어요 — {e}", "Could not create the ZIP — {e}"))?;
        let mut zw = zip::ZipWriter::new(f);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);
        for (slug, dir) in skills {
            if !dir.is_dir() {
                let _ = fs::remove_file(&tmp);
                return Err(tr!("{slug} 폴더를 찾지 못했어요", "Could not find the {slug} folder"));
            }
            for rel in fsutil::list_files(dir).map_err(|e| e.to_string())? {
                zw.start_file(format!("{slug}/{rel}"), opts).map_err(|e| e.to_string())?;
                let bytes = fs::read(join_rel(dir, &rel)).map_err(|e| e.to_string())?;
                zw.write_all(&bytes).map_err(|e| e.to_string())?;
            }
            count += 1;
        }
        zw.finish().map_err(|e| e.to_string())?;
    }
    let bytes = fs::metadata(&tmp).map_err(|e| e.to_string())?.len();
    if bytes > MAX_BYTES {
        let _ = fs::remove_file(&tmp);
        let mb = bytes as f64 / (1024.0 * 1024.0);
        return Err(tr!(
            "공유할 수 있는 크기(2MB)를 넘었어요 ({mb:.1}MB) — 스킬을 나눠서 보내거나, 큰 파일을 뺀 뒤 다시 해 보세요",
            "That is over the 2 MB sharing limit ({mb:.1} MB) — share fewer skills, or remove the large files and try again"
        ));
    }
    fs::rename(&tmp, out).map_err(|e| e.to_string())?;
    Ok(Bundle { bytes, count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;

    #[test]
    fn reads_codes_and_links() {
        for s in ["AB3K9MZ", " ab3k9mz ", "AB3K9MZ/", "https://skills-library-share.sakugarchive.workers.dev/bundles/AB3K9MZ"] {
            assert_eq!(parse_code(s).unwrap(), "AB3K9MZ", "{s}");
        }
        assert_eq!(
            parse_code("https://skills-library-share.sakugarchive.workers.dev/BUNDLES/ab3k9mz?x=1#y").unwrap(),
            "AB3K9MZ"
        );
        assert_eq!(share_url("AB3K9MZ"), "https://skills-library-share.sakugarchive.workers.dev/bundles/AB3K9MZ");
    }

    #[test]
    fn rejects_what_is_not_a_code() {
        for s in ["", "   ", "AB3K9M", "AB3K9MZZ", "AB3K9-Z", "https://github.com/anthropics/skills", "https://github.com/o/r/tree/main/x"] {
            assert!(parse_code(s).is_err(), "{s}");
            assert!(!looks_like_code(s), "{s}");
        }
        assert!(looks_like_code("https://skills-library-share.sakugarchive.workers.dev/bundles/AB3K9MZ"));
    }

    #[test]
    fn packs_the_chosen_skills() {
        let t = TestDir::new("share-pack");
        t.write("a/SKILL.md", "---\nname: a\ndescription: d\n---\n");
        t.write("a/scripts/run.py", "print(1)");
        t.write("b/SKILL.md", "---\nname: b\ndescription: d\n---\n");
        t.write("b/.DS_Store", "skip me");
        let out = t.path().join("out/bundle.zip");
        let skills = vec![
            ("a".to_string(), t.path().join("a")),
            ("b".to_string(), t.path().join("b")),
        ];
        let b = pack(&skills, &out).unwrap();
        assert_eq!(b.count, 2);
        assert_eq!(b.bytes, fs::metadata(&out).unwrap().len());

        // 번들 모양은 앱의 ZIP 가져오기와 같아요 — 풀면 스킬 폴더 둘이 나와요.
        let roots = crate::zipio::extract(&out, &t.path().join("x")).unwrap();
        let names: Vec<_> = roots.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, vec!["a", "b"]);
        assert!(t.path().join("x/a/scripts/run.py").is_file());
        assert!(!t.path().join("x/b/.DS_Store").exists());
    }

    #[test]
    fn refuses_empty_selection_and_missing_folder() {
        let t = TestDir::new("share-empty");
        assert!(pack(&[], &t.path().join("o.zip")).is_err());
        let one = vec![("nope".to_string(), t.path().join("nope"))];
        assert!(pack(&one, &t.path().join("o.zip")).is_err());
        assert!(!t.path().join("o.zip").exists());
    }
}
