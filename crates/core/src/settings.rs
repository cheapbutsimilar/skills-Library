//! 설정 스키마·기본값·검증·마이그레이션.
//!
//! - camelCase JSON 하나로 저장해요.
//! - `v`로 버전을 관리하고, 예전 버전이면 `migrate`에서 올려요.
//! - `sanitize`가 모든 값의 범위를 강제해요. UI에서 이상한 값이 와도 앱이 깨지지 않아요.

use crate::targets::{builtin_targets, Agent, Target};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const VERSION: u32 = 1;
pub const PRESETS: &[&str] = &["iris", "ember", "forest", "ocean", "ink", "sand"];
pub const FONTS: &[&str] = &["pretendard", "segoe", "malgun", "noto", "system"];
pub const ZOOMS: &[u32] = &[90, 100, 110, 125];
pub const MAX_TARGETS: usize = 40;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    /// system | light | dark
    pub mode: String,
    pub preset: String,
    /// 비어 있으면 프리셋 값을 써요. 있으면 `#RRGGBB`.
    pub bg: String,
    pub text: String,
    pub accent: String,
    pub font: String,
    pub zoom: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            mode: "system".into(),
            preset: "iris".into(),
            bg: String::new(),
            text: String::new(),
            accent: String::new(),
            font: "pretendard".into(),
            zoom: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinState {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    #[serde(default)]
    pub maximized: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub v: u32,
    pub theme: Theme,
    /// 비어 있으면 기본 위치(앱 데이터 폴더/library)
    pub library_dir: String,
    pub targets: Vec<Target>,
    /// 라이브러리에서 고치면, 설치 위치에서 따로 고치지 않은 설치본을 바로 업데이트해요.
    pub auto_sync: bool,
    pub autostart: bool,
    pub auto_update: bool,
    pub close_to_tray: bool,
    pub hotkey_enabled: bool,
    pub hotkey: String,
    /// 설치 위치에 라이브러리에 없는 스킬이 새로 생기면 알려요.
    pub notify_external: bool,
    /// 라이브러리 정렬: name | updated
    pub sort: String,
    pub window: Option<WinState>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            v: VERSION,
            theme: Theme::default(),
            library_dir: String::new(),
            targets: builtin_targets(),
            auto_sync: true,
            autostart: false,
            auto_update: true,
            close_to_tray: true,
            hotkey_enabled: true,
            hotkey: "CommandOrControl+Alt+K".into(),
            notify_external: true,
            sort: "name".into(),
            window: None,
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn clean_color(s: &mut String) {
    let t = s.trim().to_ascii_uppercase();
    *s = if is_hex_color(&t) { t } else { String::new() };
}

fn one_of(value: &mut String, allowed: &[&str], fallback: &str) {
    if !allowed.contains(&value.as_str()) {
        *value = fallback.into();
    }
}

pub fn valid_hotkey(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.len() <= 64
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+')
        && !s.starts_with('+')
        && !s.ends_with('+')
        && s.contains('+')
}

impl Settings {
    pub fn sanitize(&mut self) {
        self.v = VERSION;
        let t = &mut self.theme;
        one_of(&mut t.mode, &["system", "light", "dark"], "system");
        one_of(&mut t.preset, PRESETS, "iris");
        one_of(&mut t.font, FONTS, "pretendard");
        clean_color(&mut t.bg);
        clean_color(&mut t.text);
        clean_color(&mut t.accent);
        if !ZOOMS.contains(&t.zoom) {
            t.zoom = 100;
        }
        one_of(&mut self.sort, &["name", "updated"], "name");
        if !valid_hotkey(&self.hotkey) {
            self.hotkey = Settings::default().hotkey;
        }
        self.library_dir = self.library_dir.trim().chars().take(1024).collect();
        if let Some(w) = &mut self.window {
            w.w = w.w.clamp(900, 10000);
            w.h = w.h.clamp(600, 10000);
            w.x = w.x.clamp(-20000, 20000);
            w.y = w.y.clamp(-20000, 20000);
        }
        self.sanitize_targets();
    }

    fn sanitize_targets(&mut self) {
        let mut seen = std::collections::BTreeSet::new();
        let builtins = builtin_targets();
        let mut out: Vec<Target> = Vec::new();
        for mut t in std::mem::take(&mut self.targets) {
            t.path = t.path.trim().chars().take(1024).collect();
            t.name = t.name.trim().chars().take(40).collect();
            let id_ok = !t.id.is_empty()
                && t.id.len() <= 40
                && t.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !id_ok || seen.contains(&t.id) {
                t.id = next_id(&seen);
            }
            if let Some(b) = builtins.iter().find(|b| b.id == t.id) {
                // 기본 대상은 끌 수만 있고, 이름·에이전트는 고정이에요. 경로는 바꿀 수 있어요.
                t.builtin = true;
                t.agent = b.agent;
                t.name = b.name.clone();
                if t.path.is_empty() {
                    t.path = b.path.clone();
                }
            } else {
                t.builtin = false;
            }
            if t.path.is_empty() || t.id == crate::targets::CLAUDE_APP {
                continue;
            }
            if t.name.is_empty() {
                t.name = t.agent.label().into();
            }
            seen.insert(t.id.clone());
            out.push(t);
            if out.len() >= MAX_TARGETS {
                break;
            }
        }
        for (i, b) in builtins.into_iter().enumerate() {
            if !out.iter().any(|t| t.id == b.id) {
                out.insert(i.min(out.len()), b);
            }
        }
        self.targets = out;
    }

    pub fn target(&self, id: &str) -> Option<&Target> {
        self.targets.iter().find(|t| t.id == id)
    }

    /// 새 프로젝트 대상을 만들어요 (경로가 이미 있으면 건너뛰어요).
    pub fn add_target(&mut self, name: &str, agent: Agent, path: &str) -> Option<String> {
        if self.targets.iter().any(|t| same_path(&t.path, path)) {
            return None;
        }
        let seen = self.targets.iter().map(|t| t.id.clone()).collect();
        let id = next_id(&seen);
        self.targets.push(Target {
            id: id.clone(),
            name: name.into(),
            agent,
            path: path.into(),
            enabled: true,
            builtin: false,
        });
        self.sanitize();
        Some(id)
    }
}

fn same_path(a: &str, b: &str) -> bool {
    let n = |s: &str| s.trim().trim_end_matches(['/', '\\']).replace('\\', "/").to_lowercase();
    n(a) == n(b)
}

fn next_id(seen: &std::collections::BTreeSet<String>) -> String {
    (1..).map(|i| format!("t-{i}")).find(|id| !seen.contains(id)).unwrap()
}

/// 예전 버전 설정을 지금 버전 모양으로 올려요. (v1이 첫 버전이라 지금은 할 일이 없어요.)
fn migrate(v: &mut Value) {
    let ver = v.get("v").and_then(Value::as_u64).unwrap_or(0) as u32;
    if ver < 1 {
        // v0(버전 없음) → v1: 모양은 같아요.
    }
    if let Some(o) = v.as_object_mut() {
        o.insert("v".into(), Value::from(VERSION));
    }
}

/// JSON merge patch: 객체는 재귀로 합치고, 배열·값은 바꿔요. null은 키를 지워요.
pub fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, pv) in p {
                if pv.is_null() {
                    b.remove(k);
                } else {
                    merge(b.entry(k.clone()).or_insert(Value::Null), pv);
                }
            }
        }
        (b, p) => *b = p.clone(),
    }
}

/// 저장된 JSON을 설정으로 바꿔요. 일부 키가 깨져 있으면 그 키만 기본값으로 바꿔요.
pub fn from_value(mut v: Value) -> Settings {
    migrate(&mut v);
    let mut base = serde_json::to_value(Settings::default()).unwrap();
    let mut whole = base.clone();
    merge(&mut whole, &v);
    let mut s = match serde_json::from_value::<Settings>(whole) {
        Ok(s) => s,
        Err(_) => {
            // 키 하나씩 시도해서 되는 것만 받아들여요.
            if let Value::Object(user) = v {
                for (k, val) in user {
                    let mut trial = base.clone();
                    merge(&mut trial, &Value::Object(Map::from_iter([(k, val)])));
                    if serde_json::from_value::<Settings>(trial.clone()).is_ok() {
                        base = trial;
                    }
                }
            }
            serde_json::from_value(base).unwrap_or_default()
        }
    };
    s.sanitize();
    s
}

/// 지금 설정에 부분 패치를 합쳐서 새 설정을 만들어요.
pub fn apply_patch(cur: &Settings, patch: &Value) -> Settings {
    let mut v = serde_json::to_value(cur).unwrap();
    merge(&mut v, patch);
    from_value(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_are_sane() {
        let mut s = Settings::default();
        let before = s.clone();
        s.sanitize();
        assert_eq!(s, before);
        assert!(s.auto_sync);
        assert_eq!(s.targets.len(), 3);
    }

    #[test]
    fn bad_values_are_clamped() {
        let s = from_value(json!({
            "theme": {"mode": "neon", "preset": "nope", "accent": "red", "bg": "#abcdef", "zoom": 333},
            "hotkey": "",
            "sort": "random",
            "window": {"x": 1, "y": 2, "w": 10, "h": 99999}
        }));
        assert_eq!(s.theme.mode, "system");
        assert_eq!(s.theme.preset, "iris");
        assert_eq!(s.theme.accent, "");
        assert_eq!(s.theme.bg, "#ABCDEF");
        assert_eq!(s.theme.zoom, 100);
        assert_eq!(s.hotkey, Settings::default().hotkey);
        assert_eq!(s.sort, "name");
        let w = s.window.unwrap();
        assert_eq!((w.w, w.h), (900, 10000));
    }

    #[test]
    fn broken_key_only_resets_that_key() {
        let s = from_value(json!({"autoSync": false, "targets": "garbage", "closeToTray": 5}));
        assert!(!s.auto_sync);
        assert!(s.close_to_tray);
        assert_eq!(s.targets.len(), 3);
    }

    #[test]
    fn builtin_targets_survive_and_duplicates_get_ids() {
        let s = from_value(json!({"targets": [
            {"id": "codex", "name": "hacked", "agent": "claude-code", "path": "/custom", "enabled": false},
            {"id": "x", "name": "P", "agent": "codex", "path": "/p/.agents/skills"},
            {"id": "x", "name": "Q", "agent": "codex", "path": "/q/.agents/skills"},
            {"id": "claude-app", "name": "nope", "agent": "other", "path": "/z"}
        ]}));
        let codex = s.target("codex").unwrap();
        assert_eq!((codex.name.as_str(), codex.agent, codex.path.as_str(), codex.enabled), ("Codex", Agent::Codex, "/custom", false));
        assert!(s.target("claude-code").is_some());
        assert!(s.target("claude-app").is_none());
        let ids: Vec<_> = s.targets.iter().map(|t| t.id.clone()).collect();
        let mut dedup = ids.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(ids.len(), dedup.len());
        assert_eq!(s.targets.len(), 5);
    }

    #[test]
    fn patch_merges_nested() {
        let cur = Settings::default();
        let s = apply_patch(&cur, &json!({"theme": {"mode": "dark"}}));
        assert_eq!(s.theme.mode, "dark");
        assert_eq!(s.theme.preset, "iris");
    }

    #[test]
    fn add_target_skips_same_path() {
        let mut s = Settings::default();
        assert!(s.add_target("p", Agent::Codex, "/p/.agents/skills").is_some());
        assert!(s.add_target("p", Agent::Codex, "/p/.agents/skills/").is_none());
    }

    #[test]
    fn hotkey_validation() {
        assert!(valid_hotkey("CommandOrControl+Alt+K"));
        assert!(!valid_hotkey("K"));
        assert!(!valid_hotkey("Ctrl+"));
        assert!(!valid_hotkey("Ctrl+<"));
    }
}
