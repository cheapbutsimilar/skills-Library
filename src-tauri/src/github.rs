//! GitHub URL에서 스킬 내려받기. URL 읽기·ZIP 풀기·검사는 core(`skillslib_core::github`)가 하고,
//! 여기서는 `codeload.github.com`에서 ZIP을 받아요. 업데이터와 같은 reqwest·rustls를 써요.

use skillslib_core::github::{self, GhUrl};
use skillslib_core::tr;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_BYTES: usize = 200 * 1024 * 1024;

pub struct Fetched {
    /// 스킬 폴더를 찾을 곳 (임시 폴더 안)
    pub root: PathBuf,
    pub label: String,
}

async fn download(client: &reqwest::Client, url: &str) -> Result<Option<Vec<u8>>, String> {
    let res = client.get(url).send().await.map_err(|e| tr!("GitHub에 연결하지 못했어요 — {e}", "Could not reach GitHub — {e}"))?;
    let code = res.status();
    if code == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !code.is_success() {
        let c = code.as_u16();
        return Err(tr!("GitHub가 {c} 오류를 돌려줬어요", "GitHub returned error {c}"));
    }
    if res.content_length().is_some_and(|n| n as usize > MAX_BYTES) {
        return Err(tr!("저장소가 너무 커요 (200MB까지) — 스킬 폴더 주소를 붙여 넣어 주세요", "The repository is too large (200 MB max) — paste the skill folder URL instead"));
    }
    let bytes = res.bytes().await.map_err(|e| tr!("내려받지 못했어요 — {e}", "Download failed — {e}"))?;
    if bytes.len() > MAX_BYTES {
        return Err(tr!("저장소가 너무 커요 (200MB까지) — 스킬 폴더 주소를 붙여 넣어 주세요", "The repository is too large (200 MB max) — paste the skill folder URL instead"));
    }
    Ok(Some(bytes.to_vec()))
}

/// URL의 저장소(또는 하위 폴더)를 `tmp` 아래에 풀어요.
pub async fn fetch(url: &str, tmp: &Path) -> Result<Fetched, String> {
    let gh: GhUrl = github::parse_url(url)?;
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .user_agent(concat!("skills-library/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let mut repo_found = false;
    for c in gh.candidates() {
        let Some(bytes) = download(&client, &gh.zip_url(&c.reference)).await? else { continue };
        repo_found = true;
        let dest = tmp.join(format!("gh-{}", skillslib_core::fsutil::now_ms()));
        match github::extract_subdir(&bytes, &c.subpath, &dest, &gh.repo)? {
            Some(root) => return Ok(Fetched { root, label: gh.label() }),
            None => {
                let _ = std::fs::remove_dir_all(&dest);
            }
        }
    }
    Err(if repo_found {
        tr!("저장소에서 그 폴더를 찾지 못했어요", "That folder was not found in the repository")
    } else {
        tr!(
            "저장소나 브랜치를 찾지 못했어요 — 공개 저장소인지, 주소가 맞는지 확인해 주세요",
            "Repository or branch not found — check the URL and that the repository is public"
        )
    })
}
