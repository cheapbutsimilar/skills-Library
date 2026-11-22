//! 설정이 바뀌면 무엇을 다시 할지 정해요.

use crate::commands::emit_snapshot;
use crate::{hotkeys, updater};
use skillslib_core::settings::Settings;
use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::ManagerExt;

pub fn apply(app: &AppHandle, old: &Settings, new: &Settings) {
    if old.hotkey != new.hotkey || old.hotkey_enabled != new.hotkey_enabled {
        hotkeys::register_current(app);
    }
    if old.autostart != new.autostart {
        set_autostart(app, new.autostart);
    }
    if old.library_dir != new.library_dir || old.targets != new.targets {
        emit_snapshot(app);
    }
    let _ = app.emit("settings", new);
}

pub fn set_autostart(app: &AppHandle, on: bool) {
    let al = app.autolaunch();
    let r = if on { al.enable() } else { al.disable() };
    if let Err(e) = r {
        eprintln!("자동 실행 설정 실패: {e}");
    }
}

/// 시작할 때: 릴리스 빌드에서 자동 실행이 켜져 있으면 경로를 다시 등록해요 (업데이트로 경로가 바뀌어도 유지).
pub fn on_start(app: &AppHandle, s: &Settings) {
    if updater::RELEASE && s.autostart {
        set_autostart(app, true);
    }
}
