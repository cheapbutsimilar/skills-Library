//! 브라우저 미리보기 서버.
//!
//! `ui/`를 그대로 보여주고, HTML에 `scripts/mock-api.js`(가짜 데이터 브리지)와
//! 자동 새로고침 스크립트를 끼워 넣어요. Tauri 없이 UI 작업·스크린샷을 할 수 있어요.
//!
//! ```bash
//! cargo run -p skillslib-devserver      # http://127.0.0.1:5199
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Response, Server};

const INJECT: &str = "<script src=\"/__dev/mock-api.js\"></script><script src=\"/__dev/reload.js\"></script>";

const RELOAD_JS: &str = r#"(function(){
  let last = null;
  async function tick(){
    try {
      const v = await (await fetch('/__dev/version', {cache:'no-store'})).text();
      if (last !== null && v !== last) location.reload();
      last = v;
    } catch (e) {}
    setTimeout(tick, 700);
  }
  tick();
})();"#;

fn root() -> PathBuf {
    // crates/devserver → 저장소 루트
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repo root")
}

fn mime(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn main() {
    let root = root();
    let ui = root.join("ui");
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(5199);
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).expect("서버를 열지 못했어요");
    println!("미리보기: http://{addr}/dashboard/index.html");

    for req in server.incoming_requests() {
        let url = req.url().split('?').next().unwrap_or("/").to_string();
        let resp = match url.as_str() {
            "/" => Response::from_string("").with_status_code(302).with_header(header("Location", "/dashboard/index.html")),
            "/__dev/version" => {
                let fp = skillslib_core::fsutil::fingerprint(&[ui.clone(), root.join("scripts")]);
                Response::from_string(fp.to_string()).with_header(header("Cache-Control", "no-store"))
            }
            "/__dev/reload.js" => Response::from_string(RELOAD_JS).with_header(header("Content-Type", "text/javascript")),
            "/__dev/mock-api.js" => match fs::read(root.join("scripts/mock-api.js")) {
                Ok(b) => Response::from_data(b).with_header(header("Content-Type", "text/javascript; charset=utf-8")),
                Err(_) => Response::from_string("// mock-api.js 없음").with_status_code(404),
            },
            path => {
                let rel = path.trim_start_matches('/');
                let ok = skillslib_core::fsutil::safe_rel(rel);
                let file = ui.join(rel);
                match (ok, fs::read(&file)) {
                    (true, Ok(mut bytes)) => {
                        if file.extension().is_some_and(|e| e == "html") {
                            let s = String::from_utf8_lossy(&bytes).replacen("</head>", &format!("{INJECT}</head>"), 1);
                            bytes = s.into_bytes();
                        }
                        Response::from_data(bytes)
                            .with_header(header("Content-Type", mime(&file)))
                            .with_header(header("Cache-Control", "no-store"))
                    }
                    _ => Response::from_string("없어요").with_status_code(404),
                }
            }
        };
        let _ = req.respond(resp);
    }
}
