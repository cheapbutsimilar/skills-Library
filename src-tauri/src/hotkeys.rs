//! 전역 단축키. 녹음하는 동안은 모두 풀고, 녹음이 어떻게 끝나든 다시 등록해요 (HANDOFF §5.5).

use skillslib_core::tr;
use crate::state::AppState;
use crate::windows;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

pub fn handler(app: &AppHandle, _shortcut: &Shortcut, ev: ShortcutEvent) {
    if ev.state() == ShortcutState::Pressed {
        windows::toggle_search(app);
    }
}

/// 지금 설정으로 등록해요. 녹음 중이면 건너뛰어요. 실패하면 이유를 돌려줘요.
pub fn register_current(app: &AppHandle) -> Option<String> {
    let st = app.state::<AppState>();
    if st.hotkeys_suspended.load(Ordering::SeqCst) {
        return None;
    }
    let s = st.settings();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let err = if s.hotkey_enabled {
        gs.register(s.hotkey.as_str()).err().map(|e| {
            let k = &s.hotkey;
            tr!("단축키 {k}를 등록하지 못했어요 — 다른 앱이 쓰고 있을 수 있어요 ({e})", "Could not register the shortcut {k} — another app may be using it ({e})")
        })
    } else {
        None
    };
    *st.hotkey_error.lock().unwrap() = err.clone();
    let _ = app.emit("event", serde_json::json!({ "type": "hotkey", "error": err }));
    err
}

pub fn suspend(app: &AppHandle) {
    let st = app.state::<AppState>();
    st.hotkeys_suspended.store(true, Ordering::SeqCst);
    let _ = app.global_shortcut().unregister_all();
}

/// 풀려 있었으면 다시 등록해요. 안전장치로 창이 숨거나 닫힐 때도 불러요.
pub fn resume(app: &AppHandle) -> Option<String> {
    let st = app.state::<AppState>();
    if st.hotkeys_suspended.swap(false, Ordering::SeqCst) {
        return register_current(app);
    }
    let err = st.hotkey_error.lock().unwrap().clone();
    err
}
