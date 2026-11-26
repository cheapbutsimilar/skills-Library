//! 대시보드 창 만들기·배치. 창 위치는 물리 px로 다뤄요 (HANDOFF §5.3).
//!
//! 저장 좌표는 "물리 px ÷ 주 모니터 배율" 하나로 통일하고, 되살릴 때 잡을 수 있는 부분(타이틀바)이
//! 어느 모니터에도 없으면 가장 가까운 모니터로 당겨요.

use crate::state::AppState;
use skillslib_core::settings::WinState;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const MAIN: &str = "main";
const TITLEBAR: i32 = 42;

fn desktop_scale(app: &AppHandle) -> f64 {
    app.primary_monitor().ok().flatten().map(|m| m.scale_factor()).unwrap_or(1.0).max(0.5)
}

pub fn create(app: &AppHandle, visible: bool) -> tauri::Result<WebviewWindow> {
    let saved = app.state::<AppState>().settings().window;
    let (w, h) = saved.map(|s| (s.w as f64, s.h as f64)).unwrap_or((1060.0, 720.0));
    let builder = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("dashboard/index.html".into()))
        .title("Skills Library")
        .inner_size(w, h)
        .min_inner_size(900.0, 600.0)
        .shadow(true)
        .resizable(true)
        .visible(false);
    // Windows·Linux: 프레임 없는 창 + 직접 그린 타이틀바(최소화·최대화·닫기 버튼 포함).
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false);
    // macOS: 기본 신호등 버튼을 우리 타이틀바(42px) 위에 겹쳐 둬요. 화면은 body.mac에서 왼쪽을 비우고 자체 버튼을 숨겨요.
    #[cfg(target_os = "macos")]
    let builder = builder
        .decorations(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(16.0, 15.0));
    let win = builder.build()?;
    match saved {
        Some(s) => {
            place(app, &win, s);
            if s.maximized {
                let _ = win.maximize();
            }
        }
        None => center(app, &win, w, h),
    }
    if visible {
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(win)
}

fn place(app: &AppHandle, win: &WebviewWindow, s: WinState) {
    let scale = desktop_scale(app);
    let x = (s.x as f64 * scale).round() as i32;
    let y = (s.y as f64 * scale).round() as i32;
    let width = win.outer_size().map(|z| z.width as i32).unwrap_or(1060);
    let monitors = app.available_monitors().unwrap_or_default();
    // 타이틀바 가운데 띠(가로 120px)가 어느 모니터 안에라도 걸치면 그대로 둬요.
    let grab = |mx: i32, my: i32, mw: i32, mh: i32| {
        let gx0 = x + width / 2 - 60;
        let gx1 = gx0 + 120;
        gx1 > mx && gx0 < mx + mw && y + TITLEBAR > my && y < my + mh
    };
    let ok = monitors.iter().any(|m| {
        let p = m.position();
        let z = m.size();
        grab(p.x, p.y, z.width as i32, z.height as i32)
    });
    if ok || monitors.is_empty() {
        let _ = win.set_position(PhysicalPosition::new(x, y));
        return;
    }
    // 가장 가까운 모니터 가운데로.
    let nearest = monitors.iter().min_by_key(|m| {
        let p = m.position();
        let z = m.size();
        let cx = p.x as i64 + z.width as i64 / 2;
        let cy = p.y as i64 + z.height as i64 / 2;
        (cx - x as i64).pow(2) + (cy - y as i64).pow(2)
    });
    if let Some(m) = nearest {
        let p = m.position();
        let z = m.size();
        let size = win.outer_size().unwrap_or(PhysicalSize::new(1060, 720));
        let nx = p.x + (z.width as i32 - size.width as i32).max(0) / 2;
        let ny = p.y + (z.height as i32 - size.height as i32).max(0) / 2;
        let _ = win.set_position(PhysicalPosition::new(nx, ny));
    }
}

/// 숨긴 채로 만든 창은 아직 크기를 모를 수 있어서(GTK), 요청한 크기로 직접 가운데를 계산해요.
fn center(app: &AppHandle, win: &WebviewWindow, w: f64, h: f64) {
    let Some(m) = app.primary_monitor().ok().flatten() else {
        let _ = win.center();
        return;
    };
    let s = m.scale_factor();
    let (pw, ph) = ((w * s) as i32, (h * s) as i32);
    let p = m.position();
    let z = m.size();
    let x = p.x + (z.width as i32 - pw).max(0) / 2;
    let y = p.y + (z.height as i32 - ph).max(0) / 2;
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// 지금 창 위치·크기를 기억해요 (저장은 유지 스레드가 모아서 해요).
pub fn remember(app: &AppHandle, win: &tauri::Window) {
    if win.is_minimized().unwrap_or(false) || !win.is_visible().unwrap_or(true) {
        return;
    }
    let maximized = win.is_maximized().unwrap_or(false);
    let st = app.state::<AppState>();
    let mut g = st.lock();
    let prev = g.settings.window;
    let next = if maximized {
        prev.map(|p| WinState { maximized: true, ..p })
    } else {
        let scale = desktop_scale(app);
        let wscale = win.scale_factor().unwrap_or(1.0);
        match (win.outer_position(), win.inner_size()) {
            (Ok(p), Ok(z)) => Some(WinState {
                x: (p.x as f64 / scale).round() as i32,
                y: (p.y as f64 / scale).round() as i32,
                w: ((z.width as f64 / wscale).round() as u32).max(900),
                h: ((z.height as f64 / wscale).round() as u32).max(600),
                maximized: false,
            }),
            _ => prev,
        }
    };
    if next != prev && next.is_some() {
        g.settings.window = next;
        g.win_dirty = true;
    }
}

pub fn show(app: &AppHandle) {
    let win = match app.get_webview_window(MAIN) {
        Some(w) => w,
        None => match create(app, false) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("창을 만들지 못했어요: {e}");
                return;
            }
        },
    };
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
}

/// 전역 단축키: 숨어 있으면 열고 검색칸에 커서를 둬요. 앞에 있으면 숨겨요.
pub fn toggle_search(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let front = w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) && !w.is_minimized().unwrap_or(false);
        if front {
            let _ = w.hide();
            return;
        }
    }
    show(app);
    let _ = app.emit("event", serde_json::json!({ "type": "focusSearch" }));
}
