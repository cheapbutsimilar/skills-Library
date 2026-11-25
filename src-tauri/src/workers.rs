//! 백그라운드 스레드: 폴더 변경 감지, 새 외부 스킬 알림, 창 위치 저장, 업데이트 확인.

use skillslib_core::tr;
use crate::commands::emit_snapshot;
use crate::state::AppState;
use crate::updater;
use skillslib_core::{fsutil, ops};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const WATCH_EVERY: Duration = Duration::from_secs(3);
const UPDATE_FIRST: Duration = Duration::from_secs(20);
const UPDATE_EVERY: Duration = Duration::from_secs(3 * 60 * 60);

pub fn start(app: &AppHandle) {
    let a = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(WATCH_EVERY);
        watch_tick(&a);
    });
    if updater::RELEASE {
        let a = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(UPDATE_FIRST);
            loop {
                tauri::async_runtime::block_on(updater::tick(&a));
                std::thread::sleep(UPDATE_EVERY);
            }
        });
    }
}

fn watch_tick(app: &AppHandle) {
    let st = app.state::<AppState>();
    let (roots, last, dirty) = {
        let g = st.lock();
        (st.roots(&g.settings), g.fingerprint, g.win_dirty)
    };
    if dirty {
        st.lock().win_dirty = false;
        let _ = st.save_settings();
    }
    if fsutil::fingerprint(&roots) != last {
        emit_snapshot(app);
        check_external(app);
    }
}

/// 설치 위치에 라이브러리에 없는 스킬이 새로 생겼으면 한 번만 알려요.
/// 보낸 기록은 installs.json에 남겨서 재시작·업데이트 뒤에도 다시 울리지 않아요 (HANDOFF §5.6).
pub fn check_external(app: &AppHandle) {
    let st = app.state::<AppState>();
    let s = st.settings();
    let lib = st.library();
    let keys = ops::external_keys(&lib, &st.resolved(&s));
    let fresh: Vec<_> = {
        let mut g = st.lock();
        let present: std::collections::BTreeSet<String> = keys.iter().map(|k| k.0.clone()).collect();
        let first = !g.manifest.notified_init;
        let fresh: Vec<_> = keys.iter().filter(|k| !g.manifest.notified.contains(&k.0)).cloned().collect();
        let before = g.manifest.notified.clone();
        g.manifest.notified.retain(|k| present.contains(k));
        g.manifest.notified.extend(present);
        g.manifest.notified_init = true;
        if first || before != g.manifest.notified {
            st.save_manifest_locked(&g);
        }
        if first {
            Vec::new()
        } else {
            fresh
        }
    };
    if fresh.is_empty() || !s.notify_external {
        return;
    }
    let body = if fresh.len() == 1 {
        let (place, name) = (&fresh[0].1, &fresh[0].2);
        tr!("{place}에 '{name}' 스킬이 생겼어요 — 라이브러리로 가져올 수 있어요", "New skill '{name}' in {place} — you can import it into the library")
    } else {
        let n = fresh.len();
        tr!("설치 위치에 라이브러리에 없는 스킬 {n}개가 생겼어요", "{n} new skills not in the library appeared in install locations")
    };
    let _ = app.notification().builder().title(tr!("새 스킬", "New skill")).body(body).show();
}
