//! 자동 업데이트. `SKILLSLIB_RELEASE`가 있는 빌드(릴리스 워크플로)에서만 동작해요.

use skillslib_core::tr;
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

pub const RELEASE: bool = option_env!("SKILLSLIB_RELEASE").is_some();

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    if !RELEASE {
        return Err(tr!("개발 빌드에서는 업데이트를 확인하지 않아요", "Development builds do not check for updates"));
    }
    let up = app.updater().map_err(|e| e.to_string())?;
    let found = up.check().await.map_err(|e| tr!("업데이트를 확인하지 못했어요 — {e}", "Could not check for updates — {e}"))?;
    let info = found.map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone().unwrap_or_default() });
    *app.state::<AppState>().update.lock().unwrap() = info.clone();
    let _ = app.emit("event", serde_json::json!({ "type": "update", "info": info }));
    Ok(info)
}

/// 받아서 설치하고 다시 시작해요. 설치 직전에 설정과 기록을 저장해요.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    if !RELEASE {
        return Err(tr!("개발 빌드에서는 업데이트하지 않아요", "Development builds do not update"));
    }
    let up = app.updater().map_err(|e| e.to_string())?;
    let Some(u) = up.check().await.map_err(|e| e.to_string())? else {
        return Err(tr!("이미 최신 버전이에요", "Already up to date"));
    };
    let v = &u.version;
    let msg = tr!("v{v} 받는 중이에요…", "Downloading v{v}…");
    let _ = app.emit("event", serde_json::json!({ "type": "toast", "msg": msg }));
    let bytes = u.download(|_, _| {}, || {}).await.map_err(|e| tr!("업데이트를 받지 못했어요 — {e}", "Could not download the update — {e}"))?;
    on_before_exit(app);
    u.install(bytes).map_err(|e| tr!("업데이트를 설치하지 못했어요 — {e}", "Could not install the update — {e}"))?;
    app.restart();
}

pub fn on_before_exit(app: &AppHandle) {
    app.state::<AppState>().save_all();
}

/// 유지 스레드에서: 확인하고, 자동 업데이트가 켜져 있으면 설치해요.
pub async fn tick(app: &AppHandle) {
    if let Ok(Some(_)) = check(app).await {
        if app.state::<AppState>().settings().auto_update {
            if let Err(e) = install(app).await {
                eprintln!("{e}");
            }
        }
    }
}
