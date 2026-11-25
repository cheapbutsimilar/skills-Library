//! 설치 위치(대상): 에이전트가 스킬을 읽어 가는 폴더예요.
//!
//! - Claude Code: `~/.claude/skills`, 프로젝트는 `<프로젝트>/.claude/skills`
//! - Codex: `~/.agents/skills`, 프로젝트는 `<프로젝트>/.agents/skills` (예전 위치 `$CODEX_HOME/skills`)
//! - Gemini CLI: `~/.gemini/skills`, 프로젝트는 `<프로젝트>/.gemini/skills`
//! - Grok Build: `~/.grok/skills`, 프로젝트는 `<프로젝트>/.grok/skills`
//! - Antigravity: `~/.gemini/antigravity/skills`, 프로젝트는 `<프로젝트>/.agent/skills`
//! - Cursor: `~/.cursor/skills`, 프로젝트는 `<프로젝트>/.cursor/skills`
//! - Windsurf: `~/.codeium/windsurf/skills`, 프로젝트는 `<프로젝트>/.windsurf/skills`
//! - Claude 앱(claude.ai·데스크톱): 폴더가 없어요. ZIP으로 내보내서 설정 화면에 올려요.

use crate::fsutil;
use crate::library::{has_skill_md, read_skill_dir};
use crate::validate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CLAUDE_APP: &str = "claude-app";
pub const CLAUDE_APP_URL: &str = "https://claude.ai/settings/capabilities";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    ClaudeCode,
    Codex,
    Gemini,
    Grok,
    Antigravity,
    Cursor,
    Windsurf,
    #[serde(other)]
    Other,
}

impl Agent {
    pub fn project_subdir(self) -> &'static str {
        match self {
            Agent::ClaudeCode => ".claude/skills",
            Agent::Gemini => ".gemini/skills",
            Agent::Grok => ".grok/skills",
            Agent::Antigravity => ".agent/skills",
            Agent::Cursor => ".cursor/skills",
            Agent::Windsurf => ".windsurf/skills",
            Agent::Codex | Agent::Other => ".agents/skills",
        }
    }
    pub fn label(self) -> String {
        match self {
            Agent::ClaudeCode => "Claude Code".into(),
            Agent::Codex => "Codex".into(),
            Agent::Gemini => "Gemini".into(),
            Agent::Grok => "Grok".into(),
            Agent::Antigravity => "Antigravity".into(),
            Agent::Cursor => "Cursor".into(),
            Agent::Windsurf => "Windsurf".into(),
            Agent::Other => tr!("기타", "Other"),
        }
    }

    /// 프로젝트 추가 대화상자에서 쓰는 id (`claude-code`, `codex` …)
    pub fn from_id(id: &str) -> Option<Agent> {
        Some(match id {
            "claude-code" => Agent::ClaudeCode,
            "codex" => Agent::Codex,
            "gemini" => Agent::Gemini,
            "grok" => Agent::Grok,
            "antigravity" => Agent::Antigravity,
            "cursor" => Agent::Cursor,
            "windsurf" => Agent::Windsurf,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub name: String,
    pub agent: Agent,
    pub path: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub builtin: bool,
}

fn yes() -> bool {
    true
}

pub fn builtin_targets() -> Vec<Target> {
    vec![
        Target {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            agent: Agent::ClaudeCode,
            path: "~/.claude/skills".into(),
            enabled: true,
            builtin: true,
        },
        Target {
            id: "codex".into(),
            name: "Codex".into(),
            agent: Agent::Codex,
            path: "~/.agents/skills".into(),
            enabled: true,
            builtin: true,
        },
        Target {
            id: "codex-legacy".into(),
            name: tr!("Codex (예전 위치)", "Codex (legacy location)"),
            agent: Agent::Codex,
            path: "$CODEX_HOME/skills".into(),
            enabled: false,
            builtin: true,
        },
        Target {
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            agent: Agent::Gemini,
            path: "~/.gemini/skills".into(),
            enabled: false,
            builtin: true,
        },
        Target {
            id: "grok".into(),
            name: "Grok Build".into(),
            agent: Agent::Grok,
            path: "~/.grok/skills".into(),
            enabled: false,
            builtin: true,
        },
        Target {
            id: "antigravity".into(),
            name: "Antigravity".into(),
            agent: Agent::Antigravity,
            path: "~/.gemini/antigravity/skills".into(),
            enabled: false,
            builtin: true,
        },
        Target {
            id: "cursor".into(),
            name: "Cursor".into(),
            agent: Agent::Cursor,
            path: "~/.cursor/skills".into(),
            enabled: false,
            builtin: true,
        },
        Target {
            id: "windsurf".into(),
            name: "Windsurf".into(),
            agent: Agent::Windsurf,
            path: "~/.codeium/windsurf/skills".into(),
            enabled: false,
            builtin: true,
        },
    ]
}

/// 처음 보는 기본 대상(새로 설치했거나 새 버전에서 추가된 것)을 켤지 정해요.
/// 그 에이전트를 쓰는 흔적(설정 폴더)이 있을 때만 켜요.
pub fn detect_enabled(t: &Target, env: &Env) -> bool {
    let dir = expand(&t.path, env);
    match t.id.as_str() {
        "codex-legacy" => dir.is_dir(),
        "gemini" | "grok" | "antigravity" | "cursor" | "windsurf" => dir.is_dir() || dir.parent().is_some_and(|p| p.is_dir()),
        _ => t.enabled,
    }
}

#[derive(Debug, Clone)]
pub struct Env {
    pub home: PathBuf,
    pub codex_home: Option<PathBuf>,
}

impl Env {
    pub fn from_os(home: PathBuf) -> Self {
        let codex_home = std::env::var_os("CODEX_HOME").map(PathBuf::from).filter(|p| !p.as_os_str().is_empty());
        Env { home, codex_home }
    }
}

/// `~`, `$HOME`, `$CODEX_HOME`, `%USERPROFILE%`를 풀어요.
pub fn expand(path: &str, env: &Env) -> PathBuf {
    let path = path.trim();
    let codex = env.codex_home.clone().unwrap_or_else(|| env.home.join(".codex"));
    let (base, rest) = if path == "~" {
        (Some(env.home.clone()), "")
    } else if let Some(r) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        (Some(env.home.clone()), r)
    } else if let Some(r) = path.strip_prefix("$CODEX_HOME") {
        (Some(codex), r)
    } else if let Some(r) = path.strip_prefix("$HOME") {
        (Some(env.home.clone()), r)
    } else if let Some(r) = path.strip_prefix("%USERPROFILE%") {
        (Some(env.home.clone()), r)
    } else {
        (None, path)
    };
    match base {
        Some(mut b) => {
            for part in rest.split(['/', '\\']).filter(|p| !p.is_empty()) {
                b.push(part);
            }
            b
        }
        None => PathBuf::from(rest),
    }
}

/// 홈 아래 경로는 `~/…`로 줄여서 보여줘요.
pub fn display_path(p: &Path, env: &Env) -> String {
    if let Ok(rest) = p.strip_prefix(&env.home) {
        let r = rest.to_string_lossy().replace('\\', "/");
        return if r.is_empty() { "~".into() } else { format!("~/{r}") };
    }
    p.to_string_lossy().into()
}

#[derive(Debug, Clone)]
pub struct Found {
    pub dir: String,
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub hash: String,
    pub errors: usize,
    pub symlink: bool,
}

/// 설치 위치 폴더에서 스킬(SKILL.md가 있는 하위 폴더)을 찾아요.
pub fn scan(dir: &Path) -> Vec<Found> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || fsutil::is_ignored(&name) {
            continue;
        }
        let path = e.path();
        if !path.is_dir() || !has_skill_md(&path) {
            continue;
        }
        let (p, issues, _) = read_skill_dir(&path);
        let symlink = fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false);
        out.push(Found {
            name: if p.name.is_empty() { name.clone() } else { p.name },
            description: p.description,
            hash: fsutil::hash_dir(&path).unwrap_or_default(),
            errors: issues.iter().filter(|i| i.level == validate::Level::Error).count(),
            symlink,
            dir: name,
            path,
        });
    }
    out.sort_by(|a, b| a.dir.cmp(&b.dir));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;

    fn env() -> Env {
        Env { home: PathBuf::from("/home/u"), codex_home: None }
    }

    #[test]
    fn expands_home_and_codex_home() {
        assert_eq!(expand("~/.claude/skills", &env()), PathBuf::from("/home/u/.claude/skills"));
        assert_eq!(expand("$CODEX_HOME/skills", &env()), PathBuf::from("/home/u/.codex/skills"));
        let e = Env { codex_home: Some("/opt/codex".into()), ..env() };
        assert_eq!(expand("$CODEX_HOME/skills", &e), PathBuf::from("/opt/codex/skills"));
        assert_eq!(expand("/abs/path", &env()), PathBuf::from("/abs/path"));
        assert_eq!(display_path(Path::new("/home/u/.agents/skills"), &env()), "~/.agents/skills");
    }

    #[test]
    fn detects_installed_agents() {
        let t = TestDir::new("detect");
        let env = Env { home: t.path().to_path_buf(), codex_home: None };
        let b = builtin_targets();
        let gemini = b.iter().find(|x| x.id == "gemini").unwrap();
        assert!(!detect_enabled(gemini, &env));
        std::fs::create_dir_all(t.path().join(".gemini")).unwrap();
        assert!(detect_enabled(gemini, &env));
        let cursor = b.iter().find(|x| x.id == "cursor").unwrap();
        assert!(!detect_enabled(cursor, &env));
        std::fs::create_dir_all(t.path().join(".cursor")).unwrap();
        assert!(detect_enabled(cursor, &env));
        let ag = b.iter().find(|x| x.id == "antigravity").unwrap();
        assert_eq!(expand(&ag.path, &env), t.path().join(".gemini/antigravity/skills"));
        assert!(!detect_enabled(ag, &env), "~/.gemini alone is Gemini CLI, not Antigravity");
        std::fs::create_dir_all(t.path().join(".gemini/antigravity")).unwrap();
        assert!(detect_enabled(ag, &env));
        assert_eq!(Agent::Antigravity.project_subdir(), ".agent/skills");
        assert_eq!(Agent::from_id("windsurf"), Some(Agent::Windsurf));
        let legacy = b.iter().find(|x| x.id == "codex-legacy").unwrap();
        assert!(!detect_enabled(legacy, &env));
    }

    #[test]
    fn scan_finds_skill_dirs_only() {
        let t = TestDir::new("scan");
        t.write("a/SKILL.md", "---\nname: a\ndescription: d\n---\n");
        t.write("notes/readme.md", "x");
        t.write(".hidden/SKILL.md", "x");
        let f = scan(t.path());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].dir, "a");
    }

    #[test]
    fn unknown_agent_is_other() {
        let t: Target = serde_json::from_str(r#"{"id":"x","name":"x","agent":"zed","path":"/x"}"#).unwrap();
        assert_eq!(t.agent, Agent::Other);
        let t: Target = serde_json::from_str(r#"{"id":"x","name":"x","agent":"windsurf","path":"/x"}"#).unwrap();
        assert_eq!(t.agent, Agent::Windsurf);
        let t: Target = serde_json::from_str(r#"{"id":"x","name":"x","agent":"gemini","path":"/x"}"#).unwrap();
        assert_eq!(t.agent, Agent::Gemini);
        assert!(t.enabled);
    }
}
