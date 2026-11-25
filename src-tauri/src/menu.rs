//! 트레이와 트레이 메뉴.

use skillslib_core::tr;
use crate::{commands, updater, windows};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY: &str = "tray";

fn make_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", tr!("라이브러리 열기", "Open library"), true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", tr!("업데이트 모두 적용", "Apply all updates"), true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", tr!("다시 검사", "Rescan"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", tr!("종료", "Quit"), true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&open, &sync, &refresh, &sep, &quit])
}

/// 언어가 바뀌면 트레이 메뉴를 새 글자로 바꿔요.
pub fn relabel(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY), make_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = make_menu(app)?;
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
                    let n = r.done;
                    let msg = if n == 0 { tr!("적용할 업데이트가 없어요", "No updates to apply") } else { tr!("{n}곳을 업데이트했어요", "Updated {n} location(s)") };
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
