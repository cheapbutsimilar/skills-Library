//! 화면에 보여줄 전체 상태(스냅샷)를 만들어요.

use crate::library::{LibSkill, Library};
use crate::sync::{export_status, status, Manifest, Status};
use crate::targets::{self, Agent, Env, Found, Target, CLAUDE_APP};
use crate::{fsutil, validate};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Resolved {
    pub target: Target,
    pub dir: PathBuf,
}

pub fn resolve(targets: &[Target], env: &Env) -> Vec<Resolved> {
    targets
        .iter()
        .map(|t| Resolved { target: t.clone(), dir: targets::expand(&t.path, env) })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    #[serde(flatten)]
    pub skill: LibSkill,
    /// 대상 id → 상태 (켜진 대상 + claude-app)
    pub installs: BTreeMap<String, Status>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSkill {
    pub dir: String,
    pub name: String,
    pub description: String,
    pub in_library: bool,
    pub status: Option<Status>,
    pub errors: usize,
    pub symlink: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetView {
    pub id: String,
    pub name: String,
    pub agent: Agent,
    pub path: String,
    pub full_path: String,
    pub enabled: bool,
    pub builtin: bool,
    pub exists: bool,
    pub skills: Vec<TargetSkill>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub skills: usize,
    pub with_errors: usize,
    pub installed: usize,
    pub updates: usize,
    pub attention: usize,
    pub external: usize,
    pub targets: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub library_dir: String,
    pub skills: Vec<SkillView>,
    pub targets: Vec<TargetView>,
    pub counts: Counts,
    pub claude_app_url: &'static str,
    pub at: u64,
}

/// 스냅샷을 만들어요. 내용이 같아진 설치본은 기록을 맞춰요(adopt). 기록이 바뀌었으면 `true`.
pub fn build(lib: &Library, resolved: &[Resolved], man: &mut Manifest, env: &Env) -> (Snapshot, bool) {
    let mut changed = false;
    let skills = lib.list();
    let now = fsutil::now_ms();
    let mut found: BTreeMap<String, Vec<Found>> = BTreeMap::new();
    for r in resolved.iter().filter(|r| r.target.enabled) {
        found.insert(r.target.id.clone(), targets::scan(&r.dir));
    }

    let mut counts = Counts { skills: skills.len(), ..Default::default() };
    let mut views = Vec::with_capacity(skills.len());
    for s in skills {
        if validate::has_errors(&s.issues) {
            counts.with_errors += 1;
        }
        let mut installs = BTreeMap::new();
        for r in resolved.iter().filter(|r| r.target.enabled) {
            let id = &r.target.id;
            let t_hash = found[id].iter().find(|f| f.dir == s.slug).map(|f| f.hash.as_str());
            let rec = man.get(id, &s.slug).map(|r| r.hash.clone());
            let st = status(&s.hash, t_hash, rec.as_deref());
            match st {
                Status::Synced => changed |= man.set(id, &s.slug, &s.hash, now),
                Status::NotInstalled => changed |= man.remove(id, &s.slug),
                _ => {}
            }
            if st.installed() {
                counts.installed += 1;
            }
            if st == Status::LibraryNewer {
                counts.updates += 1;
            }
            if st.needs_attention() {
                counts.attention += 1;
            }
            installs.insert(id.clone(), st);
        }
        let rec = man.get(CLAUDE_APP, &s.slug).map(|r| r.hash.clone());
        installs.insert(CLAUDE_APP.into(), export_status(&s.hash, rec.as_deref()));
        views.push(SkillView { skill: s, installs });
    }
    // 라이브러리에서 사라진 스킬의 내보내기 기록은 정리해요.
    if let Some(m) = man.installs.get_mut(CLAUDE_APP) {
        let before = m.len();
        m.retain(|slug, _| views.iter().any(|v| &v.skill.slug == slug));
        changed |= before != m.len();
    }

    let mut tviews = Vec::new();
    for r in resolved {
        let t = &r.target;
        let skills: Vec<TargetSkill> = found
            .get(&t.id)
            .map(|list| {
                list.iter()
                    .map(|f| {
                        let lib_view = views.iter().find(|v| v.skill.slug == f.dir);
                        TargetSkill {
                            dir: f.dir.clone(),
                            name: f.name.clone(),
                            description: f.description.clone(),
                            in_library: lib_view.is_some(),
                            status: lib_view.and_then(|v| v.installs.get(&t.id).copied()),
                            errors: f.errors,
                            symlink: f.symlink,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        if t.enabled {
            counts.targets += 1;
            counts.external += skills.iter().filter(|s| !s.in_library).count();
        }
        tviews.push(TargetView {
            id: t.id.clone(),
            name: t.name.clone(),
            agent: t.agent,
            path: targets::display_path(&r.dir, env),
            full_path: r.dir.to_string_lossy().into(),
            enabled: t.enabled,
            builtin: t.builtin,
            exists: r.dir.is_dir(),
            skills,
        });
    }

    let snap = Snapshot {
        library_dir: lib.dir.to_string_lossy().into(),
        skills: views,
        targets: tviews,
        counts,
        claude_app_url: targets::CLAUDE_APP_URL,
        at: now,
    };
    (snap, changed)
}
