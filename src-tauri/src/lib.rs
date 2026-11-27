//! 앱 조립: 플러그인, 창, 트레이, 워커 스레드.

mod apply;
mod commands;
mod github;
mod hotkeys;
mod menu;
mod share;
mod state;
mod tray_icon;
mod updater;
mod windows;
mod workers;

use state::AppState;
use tauri::{DragDropEvent, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 두 번째 실행은 이미 떠 있는 앱의 창을 열어요.
            windows::show(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(hotkeys::handler).build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let data = app.path().app_data_dir()?;
            let home = app.path().home_dir()?;
            app.manage(AppState::load(data, home));
            let st = app.state::<AppState>();
            st.refresh();
            if st.first_run {
                // 기본값으로 바로 쓸 수 있게: 이미 설치된 스킬을 라이브러리로 복사해 와요 (원본은 그대로).
                let r = commands::import_all_external_inner(&handle);
                st.first_imported.store(r.slugs.len(), std::sync::atomic::Ordering::SeqCst);
            }
            menu::build(&handle)?;
            if let Some(snap) = st.lock().snapshot.clone() {
                tray_icon::update(&handle, &snap.counts);
            }
            let hidden = std::env::args().any(|a| a == "--hidden");
            windows::create(&handle, !hidden)?;
            hotkeys::register_current(&handle);
            apply::on_start(&handle, &st.settings());
            workers::check_external(&handle);
            workers::start(&handle);
            Ok(())
        })
        .on_window_event(|win, ev| {
            let app = win.app_handle();
            match ev {
                WindowEvent::CloseRequested { api, .. } => {
                    windows::remember(app, win);
                    let st = app.state::<AppState>();
                    let _ = st.save_settings();
                    hotkeys::resume(app);
                    if st.settings().close_to_tray {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                }
                WindowEvent::Destroyed => {
                    // 녹음 중에 창이 사라져도 단축키는 되살려요.
                    hotkeys::resume(app);
                }
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => windows::remember(app, win),
                WindowEvent::DragDrop(DragDropEvent::Enter { .. }) | WindowEvent::DragDrop(DragDropEvent::Over { .. }) => {
                    let _ = app.emit("event", serde_json::json!({ "type": "drag", "over": true }));
                }
                WindowEvent::DragDrop(DragDropEvent::Leave) => {
                    let _ = app.emit("event", serde_json::json!({ "type": "drag", "over": false }));
                }
                WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) => {
                    let _ = app.emit("event", serde_json::json!({ "type": "drag", "over": false }));
                    let app = app.clone();
                    let paths = paths.clone();
                    std::thread::spawn(move || {
                        let r = commands::import_paths_inner(&app, &paths);
                        let _ = app.emit("event", serde_json::json!({ "type": "imported", "slugs": r.slugs, "errors": r.errors }));
                    });
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::init,
            commands::settings_set,
            commands::refresh,
            commands::skill_get,
            commands::skill_check,
            commands::skill_save,
            commands::skill_save_fields,
            commands::skill_create,
            commands::skill_duplicate,
            commands::skill_delete,
            commands::skill_meta,
            commands::file_read,
            commands::file_write,
            commands::file_delete,
            commands::install,
            commands::uninstall,
            commands::pull,
            commands::import_target,
            commands::import_all_external,
            commands::sync_all,
            commands::cleanup_plan,
            commands::cleanup_apply,
            commands::import_pick,
            commands::import_paths,
            commands::import_github,
            commands::share_upload,
            commands::import_share,
            commands::diff,
            commands::install_all,
            commands::set_lang,
            commands::export_zip,
            commands::target_add_project,
            commands::target_pick_path,
            commands::target_reset_path,
            commands::target_remove,
            commands::library_change_dir,
            commands::open_path,
            commands::open_skill_dir,
            commands::open_special,
            commands::open_url,
            commands::win_minimize,
            commands::win_toggle_maximize,
            commands::win_close,
            commands::hotkey_suspend,
            commands::hotkey_resume,
            commands::update_check,
            commands::update_install,
        ])
        .build(tauri::generate_context!())
        .expect("앱을 시작하지 못했어요");

    app.run(|app, ev| match ev {
        RunEvent::ExitRequested { .. } => updater::on_before_exit(app),
        // macOS: 창을 닫아(숨겨) 둔 채 Dock 아이콘을 누르면 다시 열어요.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { has_visible_windows: false, .. } => windows::show(app),
        _ => {}
    });
}
