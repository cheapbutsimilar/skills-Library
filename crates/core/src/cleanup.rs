//! 정리: 흩어진 스킬을 한 번에 맞춰요.
//!
//! 꼬이지 않게 순서를 지켜요:
//! 1. 라이브러리에 없는 스킬을 가져와요.
//! 2. 이름이 같은데 내용이 다른 스킬은 **버전 하나를 골라** 라이브러리와 설치된 곳 전부를 그 버전으로 맞춰요.
//!    (내용을 섞는 병합은 하지 않아요 — 섞으면 어느 쪽도 아닌 스킬이 돼요.)
//! 3. 예전 Codex 위치의 스킬을 현 위치로 옮겨요. 이때는 내용이 같은 것만 옮기고, 다른 건 남겨요.
//!
//! 밀려나는 내용은 모두 휴지통에 백업해요.

use crate::library::{read_skill_dir, Library};
use crate::ops::{self, Ctx};
use crate::snapshot::Resolved;
use crate::sync::{status, Manifest};
use crate::targets::{self, scan, Agent, Env, Found};
use crate::fsutil;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const LEGACY_FROM: &str = "codex-legacy";
pub const LEGACY_TO: &str = "codex";
pub const LIBRARY: &str = "library";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    /// "library" 또는 대상 id
    pub id: String,
    pub name: String,
    pub agent: Option<Agent>,
}

/// 내용이 같은 복사본 묶음
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub hash: String,
    pub files: usize,
    pub bytes: u64,
    pub updated: u64,
    pub places: Vec<Place>,
    pub in_library: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Divergent {
    pub slug: String,
    pub description: String,
    /// 최근에 고친 순서
    pub versions: Vec<Version>,
    /// 기본으로 고를 버전 (가장 최근에 고친 것)
    pub suggested: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LegacyState {
    /// 현 위치에 없어요 → 옮겨요
    Move,
    /// 현 위치에 같은 내용이 있어요 → 예전 것만 치워요
    Duplicate,
    /// 현 위치의 내용과 달라요 → 먼저 버전을 골라야 해요
    Differs,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacySkill {
    pub dir: String,
    pub state: LegacyState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyPlan {
    pub from_name: String,
    pub from_path: String,
    pub to_name: String,
    pub to_path: String,
    pub skills: Vec<LegacySkill>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub divergent: Vec<Divergent>,
    pub legacy: Option<LegacyPlan>,
    pub external: usize,
}

fn add_version(versions: &mut Vec<Version>, hash: &str, dir: &Path, place: Place) {
    let st = fsutil::dir_stats(dir);
    let is_lib = place.id == LIBRARY;
    match versions.iter_mut().find(|v| v.hash == hash) {
        Some(v) => {
            v.updated = v.updated.max(st.updated);
            v.in_library |= is_lib;
            v.places.push(place);
        }
        None => versions.push(Version {
            hash: hash.into(),
            files: st.files,
            bytes: st.bytes,
            updated: st.updated,
            places: vec![place],
            in_library: is_lib,
        }),
    }
}

/// 예전 위치 → 현 위치 쌍 (둘 다 켜져 있고 서로 다른 폴더일 때만)
fn legacy_pair(targets: &[Resolved]) -> Option<(&Resolved, &Resolved)> {
    let from = targets.iter().find(|r| r.target.id == LEGACY_FROM && r.target.enabled)?;
    let to = targets.iter().find(|r| r.target.id == LEGACY_TO && r.target.enabled)?;
    (from.dir != to.dir).then_some((from, to))
}

pub fn plan(lib: &Library, targets: &[Resolved], man: &Manifest, env: &Env) -> Plan {
    let found: Vec<(&Resolved, Vec<Found>)> =
        targets.iter().filter(|r| r.target.enabled).map(|r| (r, scan(&r.dir))).collect();
    let slugs = lib.slugs();

    let mut divergent = Vec::new();
    for slug in &slugs {
        let lpath = lib.path(slug);
        let Ok(lh) = fsutil::hash_dir(&lpath) else { continue };
        let copies: Vec<(&Resolved, &Found)> = found
            .iter()
            .filter_map(|(r, list)| list.iter().find(|f| &f.dir == slug).map(|f| (*r, f)))
            .collect();
        let attention = copies.iter().any(|(r, f)| {
            status(&lh, Some(&f.hash), man.get(&r.target.id, slug).map(|x| x.hash.as_str())).needs_attention()
        });
        if !attention {
            continue;
        }
        let mut versions = Vec::new();
        add_version(&mut versions, &lh, &lpath, Place { id: LIBRARY.into(), name: "라이브러리".into(), agent: None });
        for (r, f) in copies {
            let place = Place { id: r.target.id.clone(), name: r.target.name.clone(), agent: Some(r.target.agent) };
            add_version(&mut versions, &f.hash, &f.path, place);
        }
        // 최근 것 먼저. 같은 시각이면 라이브러리 것이 앞에 와요(안정 정렬).
        versions.sort_by_key(|v| std::cmp::Reverse(v.updated));
        divergent.push(Divergent {
            slug: slug.clone(),
            description: read_skill_dir(&lpath).0.description,
            suggested: versions[0].hash.clone(),
            versions,
        });
    }

    let legacy = legacy_pair(targets).and_then(|(from, to)| {
        let list = |r: &Resolved| found.iter().find(|(x, _)| x.target.id == r.target.id).map(|(_, l)| l);
        let (fl, tl) = (list(from)?, list(to)?);
        if fl.is_empty() {
            return None;
        }
        let skills = fl
            .iter()
            .map(|f| LegacySkill {
                dir: f.dir.clone(),
                state: match tl.iter().find(|t| t.dir == f.dir) {
                    None => LegacyState::Move,
                    Some(t) if t.hash == f.hash => LegacyState::Duplicate,
                    Some(_) => LegacyState::Differs,
                },
            })
            .collect();
        Some(LegacyPlan {
            from_name: from.target.name.clone(),
            from_path: targets::display_path(&from.dir, env),
            to_name: to.target.name.clone(),
            to_path: targets::display_path(&to.dir, env),
            skills,
        })
    });

    let external = found.iter().map(|(_, l)| l.iter().filter(|f| !slugs.contains(&f.dir)).count()).sum();
    Plan { divergent, legacy, external }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Choices {
    /// 스킬 → 고른 버전의 해시. 없거나 비어 있으면 그대로 둬요.
    pub picks: BTreeMap<String, String>,
    pub legacy: bool,
    pub external: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub imported: usize,
    pub unified: usize,
    pub moved: usize,
    pub duplicates: usize,
    /// 건드리지 않고 남긴 것과 그 이유
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
    /// 예전 위치가 비었어요 → 꺼도 돼요
    pub legacy_emptied: bool,
}

fn hash_of(dir: &Path) -> Option<String> {
    if dir.is_dir() {
        fsutil::hash_dir(dir).ok()
    } else {
        None
    }
}

/// 고른 버전으로 라이브러리와 설치된 곳 전부를 맞춰요.
fn unify(ctx: &mut Ctx, slug: &str, want: &str, targets: &[Resolved]) -> Result<(), String> {
    if !ctx.lib.exists(slug) {
        return Err("라이브러리에 없어요".into());
    }
    let enabled: Vec<&Resolved> = targets.iter().filter(|r| r.target.enabled).collect();
    if hash_of(&ctx.lib.path(slug)).as_deref() != Some(want) {
        let src = enabled
            .iter()
            .find(|r| hash_of(&r.dir.join(slug)).as_deref() == Some(want))
            .ok_or("고른 버전을 찾지 못했어요 — 그 사이에 바뀌었어요. 다시 열어 주세요")?;
        ops::pull(ctx, slug, src)?;
    }
    for r in enabled {
        match hash_of(&r.dir.join(slug)) {
            Some(h) if h != want => ops::install(ctx, slug, r)?,
            _ => {}
        }
    }
    Ok(())
}

pub fn apply(ctx: &mut Ctx, targets: &[Resolved], c: &Choices) -> Report {
    let mut rep = Report::default();

    if c.external {
        let (slugs, errs) = ops::import_all_external(ctx, targets);
        rep.imported = slugs.len();
        rep.errors.extend(errs);
    }

    for (slug, want) in c.picks.iter().filter(|(_, h)| !h.is_empty()) {
        match unify(ctx, slug, want, targets) {
            Ok(()) => rep.unified += 1,
            Err(e) => rep.errors.push(format!("{slug}: {e}")),
        }
    }

    if c.legacy {
        if let Some((from, to)) = legacy_pair(targets) {
            for f in scan(&from.dir) {
                let slug = &f.dir;
                let Some(lh) = hash_of(&ctx.lib.path(slug)) else {
                    rep.skipped.push(format!("{slug}: 라이브러리에 없어서 그대로 뒀어요"));
                    continue;
                };
                let there = hash_of(&to.dir.join(slug));
                if f.hash != lh || there.as_ref().is_some_and(|h| *h != lh) {
                    rep.skipped.push(format!("{slug}: 내용이 서로 달라서 그대로 뒀어요 — 버전을 먼저 골라 주세요"));
                    continue;
                }
                if there.is_none() {
                    if let Err(e) = ops::install(ctx, slug, to) {
                        rep.errors.push(e);
                        continue;
                    }
                }
                match ops::uninstall(ctx, slug, from) {
                    Ok(()) if there.is_none() => rep.moved += 1,
                    Ok(()) => rep.duplicates += 1,
                    Err(e) => rep.errors.push(e),
                }
            }
            rep.legacy_emptied = scan(&from.dir).is_empty();
        }
    }
    rep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;
    use crate::snapshot::resolve;
    use crate::targets::builtin_targets;
    use std::fs;

    struct W {
        t: TestDir,
        lib: Library,
        man: Manifest,
        targets: Vec<Resolved>,
        env: Env,
    }

    fn skill(dir: &Path, name: &str, body: &str) {
        fs::create_dir_all(dir.join(name)).unwrap();
        fs::write(dir.join(name).join("SKILL.md"), format!("---\nname: {name}\ndescription: 설명이에요\n---\n{body}\n")).unwrap();
    }

    fn world() -> W {
        let t = TestDir::new("cleanup");
        let env = Env { home: t.path().join("home"), codex_home: None };
        let mut ts = builtin_targets();
        for x in ts.iter_mut() {
            x.enabled = matches!(x.id.as_str(), "claude-code" | "codex" | "codex-legacy");
        }
        let targets = resolve(&ts, &env);
        let lib = Library::new(t.path().join("lib"));
        fs::create_dir_all(&lib.dir).unwrap();
        W { t, lib, man: Manifest::default(), targets, env }
    }

    impl W {
        fn dir(&self, id: &str) -> std::path::PathBuf {
            self.targets.iter().find(|r| r.target.id == id).unwrap().dir.clone()
        }
        fn run(&mut self, c: &Choices) -> Report {
            let trash = self.t.path().join("trash");
            let targets = self.targets.clone();
            let mut ctx = Ctx { lib: &self.lib, trash: &trash, man: &mut self.man };
            apply(&mut ctx, &targets, c)
        }
        fn plan(&self) -> Plan {
            plan(&self.lib, &self.targets, &self.man, &self.env)
        }
    }

    #[test]
    fn plan_groups_versions_and_lists_legacy() {
        let w = world();
        skill(&w.lib.dir, "alpha", "라이브러리 버전");
        skill(&w.dir("claude-code"), "alpha", "클로드 버전");
        skill(&w.dir("codex-legacy"), "alpha", "라이브러리 버전");
        skill(&w.dir("codex-legacy"), "beta", "예전에만 있어요");
        skill(&w.dir("codex"), "alpha", "코덱스 버전");

        let p = w.plan();
        assert_eq!(p.divergent.len(), 1);
        let d = &p.divergent[0];
        assert_eq!(d.versions.len(), 3);
        let libv = d.versions.iter().find(|v| v.in_library).unwrap();
        assert_eq!(libv.places.len(), 2, "라이브러리와 예전 위치가 같은 버전으로 묶여요");
        assert_eq!(p.external, 1);
        let l = p.legacy.unwrap();
        assert_eq!(l.from_path, "~/.codex/skills");
        let states: Vec<_> = l.skills.iter().map(|s| (s.dir.as_str(), s.state)).collect();
        assert_eq!(states, vec![("alpha", LegacyState::Differs), ("beta", LegacyState::Move)]);
    }

    #[test]
    fn unify_then_move_legacy() {
        let mut w = world();
        skill(&w.lib.dir, "alpha", "라이브러리 버전");
        skill(&w.dir("claude-code"), "alpha", "클로드 버전");
        skill(&w.dir("codex-legacy"), "alpha", "라이브러리 버전");
        skill(&w.dir("codex-legacy"), "beta", "예전에만 있어요");
        let want = fsutil::hash_dir(&w.dir("claude-code").join("alpha")).unwrap();

        let c = Choices { picks: BTreeMap::from([("alpha".to_string(), want.clone())]), legacy: true, external: true };
        let rep = w.run(&c);
        assert!(rep.errors.is_empty(), "{:?}", rep.errors);
        assert!(rep.skipped.is_empty(), "{:?}", rep.skipped);
        assert_eq!((rep.imported, rep.unified, rep.moved), (1, 1, 2));
        assert!(rep.legacy_emptied);

        // 라이브러리·Claude Code·Codex(현 위치)가 모두 고른 버전이에요.
        for d in [w.lib.path("alpha"), w.dir("claude-code").join("alpha"), w.dir("codex").join("alpha")] {
            assert_eq!(fsutil::hash_dir(&d).unwrap(), want);
        }
        assert!(w.dir("codex").join("beta").is_dir());
        assert!(!w.dir("codex-legacy").join("alpha").exists());
        // 밀려난 라이브러리 버전은 휴지통에 남아요.
        let trash = w.t.path().join("trash");
        let kept = fs::read_dir(&trash).unwrap().flatten().any(|e| {
            fs::read_to_string(e.path().join("alpha/SKILL.md")).is_ok_and(|s| s.contains("라이브러리 버전"))
        });
        assert!(kept);
        // 다시 보면 정리할 게 없어요.
        let p = w.plan();
        assert!(p.divergent.is_empty() && p.legacy.is_none() && p.external == 0);
    }

    #[test]
    fn legacy_skips_what_differs_and_removes_duplicates() {
        let mut w = world();
        skill(&w.lib.dir, "same", "같아요");
        skill(&w.dir("codex"), "same", "같아요");
        skill(&w.dir("codex-legacy"), "same", "같아요");
        skill(&w.lib.dir, "diff", "라이브러리");
        skill(&w.dir("codex-legacy"), "diff", "예전 것은 달라요");

        let rep = w.run(&Choices { legacy: true, ..Default::default() });
        assert_eq!((rep.moved, rep.duplicates, rep.skipped.len()), (0, 1, 1));
        assert!(!rep.legacy_emptied);
        assert!(w.dir("codex-legacy").join("diff").is_dir(), "내용이 다른 건 건드리지 않아요");
        assert!(!w.dir("codex").join("diff").exists());
    }

    #[test]
    fn stale_pick_is_reported_not_guessed() {
        let mut w = world();
        skill(&w.lib.dir, "alpha", "라이브러리");
        skill(&w.dir("claude-code"), "alpha", "다른 것");
        let c = Choices { picks: BTreeMap::from([("alpha".to_string(), "deadbeef".to_string())]), ..Default::default() };
        let rep = w.run(&c);
        assert_eq!(rep.unified, 0);
        assert_eq!(rep.errors.len(), 1);
        assert!(fs::read_to_string(w.lib.path("alpha").join("SKILL.md")).unwrap().contains("라이브러리"));
    }
}
