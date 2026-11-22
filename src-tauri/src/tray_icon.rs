//! 트레이 아이콘을 직접 그려요: 책꽂이 모양 + 할 일이 있으면 오른쪽 위에 점.

use skillslib_core::snapshot::Counts;
use tauri::image::Image;
use tauri::AppHandle;

const N: u32 = 32;
const ACCENT: [u8; 3] = [0x8B, 0x7C, 0xF6];
const DOT: [u8; 3] = [0xFF, 0x45, 0x3A];

fn put(buf: &mut [u8], x: u32, y: u32, c: [u8; 3], a: u8) {
    if x >= N || y >= N {
        return;
    }
    let i = ((y * N + x) * 4) as usize;
    let sa = a as f32 / 255.0;
    let da = buf[i + 3] as f32 / 255.0;
    let oa = sa + da * (1.0 - sa);
    for k in 0..3 {
        let v = (c[k] as f32 * sa + buf[i + k] as f32 * da * (1.0 - sa)) / oa.max(1e-6);
        buf[i + k] = v.round() as u8;
    }
    buf[i + 3] = (oa * 255.0).round() as u8;
}

/// 모서리가 둥근 사각형 (가장자리 안티에일리어싱)
fn rrect(buf: &mut [u8], x0: f32, y0: f32, x1: f32, y1: f32, r: f32, c: [u8; 3]) {
    for y in 0..N {
        for x in 0..N {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let cx = px.clamp(x0 + r, x1 - r);
            let cy = py.clamp(y0 + r, y1 - r);
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() - r;
            let inside = px >= x0 - 0.5 && px <= x1 + 0.5 && py >= y0 - 0.5 && py <= y1 + 0.5;
            let a = (0.5 - d).clamp(0.0, 1.0);
            if inside && a > 0.0 {
                put(buf, x, y, c, (a * 255.0) as u8);
            }
        }
    }
}

fn circle(buf: &mut [u8], cx: f32, cy: f32, r: f32, c: [u8; 3]) {
    for y in 0..N {
        for x in 0..N {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt() - r;
            let a = (0.5 - d).clamp(0.0, 1.0);
            if a > 0.0 {
                put(buf, x, y, c, (a * 255.0) as u8);
            }
        }
    }
}

pub fn draw(badge: bool) -> Vec<u8> {
    let mut buf = vec![0u8; (N * N * 4) as usize];
    rrect(&mut buf, 2.0, 2.0, 30.0, 30.0, 7.0, ACCENT);
    let white = [0xFF, 0xFF, 0xFF];
    // 책 세 권 + 기울어진 한 권 느낌의 막대
    rrect(&mut buf, 8.0, 9.0, 11.0, 24.0, 1.0, white);
    rrect(&mut buf, 13.0, 12.0, 16.0, 24.0, 1.0, white);
    rrect(&mut buf, 18.0, 8.0, 21.0, 24.0, 1.0, white);
    rrect(&mut buf, 7.0, 25.0, 25.0, 26.5, 0.5, white);
    if badge {
        circle(&mut buf, 25.5, 6.5, 6.0, [0xFF, 0xFF, 0xFF]);
        circle(&mut buf, 25.5, 6.5, 4.5, DOT);
    }
    buf
}

pub fn image(badge: bool) -> Image<'static> {
    Image::new_owned(draw(badge), N, N)
}

pub fn update(app: &AppHandle, c: &Counts) {
    let Some(tray) = app.tray_by_id(crate::menu::TRAY) else { return };
    let badge = c.updates > 0 || c.attention > 0;
    let _ = tray.set_icon(Some(image(badge)));
    let mut tip = format!("Skills Library — 스킬 {}개", c.skills);
    if c.updates > 0 {
        tip.push_str(&format!(" · 업데이트 {}개", c.updates));
    }
    if c.attention > 0 {
        tip.push_str(&format!(" · 확인 필요 {}개", c.attention));
    }
    let _ = tray.set_tooltip(Some(tip));
}
