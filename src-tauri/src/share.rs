//! 공유 번들 올리기·내려받기. 코드 읽기와 ZIP 묶기는 core(`skillslib_core::share`)가 하고,
//! 여기서는 Cloudflare Workers API(`/upload`, `/bundles/{code}`)와 이야기해요.
//! 업데이터·GitHub 가져오기와 같은 reqwest·rustls를 써요.

use serde::Deserialize;
use skillslib_core::share;
use skillslib_core::tr;
use std::time::Duration;

/// 내려받기 사고를 막으려는 상한. 서버가 2MB만 두지만, 이상한 응답은 여기서 끊어요.
const MAX_DOWNLOAD: usize = 4 * 1024 * 1024;

pub struct Published {
    pub code: String,
    pub url: String,
    pub expires_in_days: u32,
}

#[derive(Deserialize)]
struct UploadResponse {
    code: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default, rename = "expiresInDays")]
    expires_in_days: Option<u32>,
}

fn client() -> Result<reqwest::Client, String> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .user_agent(concat!("skills-library/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}

fn status_error(code: u16) -> String {
    match code {
        429 => tr!(
            "잠시 뒤에 다시 해 주세요 — 공유 서버가 짧은 시간에 너무 많은 요청을 받았어요",
            "Please try again in a moment — the share server is getting too many requests"
        ),
        413 => tr!(
            "공유할 수 있는 크기(2MB)를 넘었어요 — 스킬을 나눠서 보내 주세요",
            "That is over the 2 MB sharing limit — share fewer skills"
        ),
        _ => tr!("공유 서버가 {code} 오류를 돌려줬어요", "The share server returned error {code}"),
    }
}

/// ZIP을 올리고 7자리 코드를 받아요.
pub async fn upload(zip: Vec<u8>) -> Result<Published, String> {
    if zip.is_empty() {
        return Err(tr!("보낼 내용이 비어 있어요", "There is nothing to send"));
    }
    let res = client()?
        .post(format!("{}/upload", share::BASE_URL))
        .header(reqwest::header::CONTENT_TYPE, "application/zip")
        .body(zip)
        .send()
        .await
        .map_err(|e| tr!("공유 서버에 연결하지 못했어요 — {e}", "Could not reach the share server — {e}"))?;
    let code = res.status();
    let body = res.bytes().await.map_err(|e| tr!("응답을 읽지 못했어요 — {e}", "Could not read the response — {e}"))?;
    if !code.is_success() {
        return Err(status_error(code.as_u16()));
    }
    let r: UploadResponse = serde_json::from_slice(&body)
        .map_err(|_| tr!("공유 서버가 예상과 다른 응답을 보냈어요", "The share server sent an unexpected response"))?;
    let shared = share::parse_code(&r.code)?;
    Ok(Published {
        url: r.url.unwrap_or_else(|| share::share_url(&shared)),
        code: shared,
        expires_in_days: r.expires_in_days.unwrap_or(share::EXPIRES_IN_DAYS),
    })
}

/// 코드로 ZIP을 내려받아요.
pub async fn download(code: &str) -> Result<Vec<u8>, String> {
    let res = client()?
        .get(share::share_url(code))
        .send()
        .await
        .map_err(|e| tr!("공유 서버에 연결하지 못했어요 — {e}", "Could not reach the share server — {e}"))?;
    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(tr!(
            "그 코드를 찾지 못했어요 — 코드가 맞는지, 30일이 지나 만료되지 않았는지 확인해 주세요",
            "That code was not found — check the code, and that it has not expired after 30 days"
        ));
    }
    if !status.is_success() {
        return Err(status_error(status.as_u16()));
    }
    if res.content_length().is_some_and(|n| n as usize > MAX_DOWNLOAD) {
        return Err(tr!("받은 파일이 너무 커요", "The download is too large"));
    }
    let bytes = res.bytes().await.map_err(|e| tr!("내려받지 못했어요 — {e}", "Download failed — {e}"))?;
    if bytes.len() > MAX_DOWNLOAD {
        return Err(tr!("받은 파일이 너무 커요", "The download is too large"));
    }
    Ok(bytes.to_vec())
}
