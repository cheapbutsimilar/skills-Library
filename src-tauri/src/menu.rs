//! 트레이와 트레이 메뉴.

use crate::{commands, updater, windows};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY: &str = "tray";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "라이브러리 열기", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "업데이트 모두 적용", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "다시 검사", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &sync, &refresh, &sep, &quit])?;
    TrayIconBuilder::with_id(TRAY)
        .icon(crate::tray_icon::image(false))
        .tooltip("Skills Library")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => windows::show(app),
            "sync" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    let st = app.state::<crate::state::AppState>();
                    let r = commands::sync_all_inner(&app, &st);
                    let msg = if r.done == 0 { "적용할 업데이트가 없어요".to_string() } else { format!("{}곳을 업데이트했어요", r.done) };
                    commands::toast(&app, &msg);
                });
            }
            "refresh" => {
                commands::emit_snapshot(app);
            }
            "quit" => {
                updater::on_before_exit(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                windows::show(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
