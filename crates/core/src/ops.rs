//! 설치·업데이트·제거·가져오기 같은 동작. 모두 "지우지 않고 휴지통으로" 원칙을 따라요.

use crate::fsutil;
use crate::library::Library;
use crate::snapshot::Resolved;
use crate::sync::{status, Manifest, Status};
use crate::targets::scan;
use std::path::{Path, PathBuf};

pub struct Ctx<'a> {
    pub lib: &'a Library,
    pub trash: &'a Path,
    pub man: &'a mut Manifest,
}

fn dest(r: &Resolved, slug: &str) -> PathBuf {
    r.dir.join(slug)
}

fn lib_hash(lib: &Library, slug: &str) -> Result<String, String> {
    if !lib.exists(slug) {
        return Err(tr!("라이브러리에 그 스킬이 없어요", "That skill is not in the library"));
    }
    fsutil::hash_dir(&lib.path(slug)).map_err(|e| tr!("스킬을 읽지 못했어요 — {e}", "Could not read the skill — {e}"))
}

pub fn current_status(ctx: &Ctx, slug: &str, r: &Resolved) -> Result<Status, String> {
    let lh = lib_hash(ctx.lib, slug)?;
    let d = dest(r, slug);
    let th = if d.is_dir() { fsutil::hash_dir(&d).ok() } else { None };
    Ok(status(&lh, th.as_deref(), ctx.man.get(&r.target.id, slug).map(|r| r.hash.as_str())))
}

/// 라이브러리 스킬을 설치 위치에 복사해요 (있으면 바꿔요). 덮이는 내용이 라이브러리와 다르면 휴지통에 백업해요.
pub fn install(ctx: &mut Ctx, slug: &str, r: &Resolved) -> Result<(), String> {
    let lh = lib_hash(ctx.lib, slug)?;
    let d = dest(r, slug);
    if d.is_dir() {
        let st = current_status(ctx, slug, r)?;
        if st.needs_attention() {
            let bak = ctx.trash.join(format!("{}-{}-{slug}-overwritten", fsutil::now_ms(), r.target.id));
            fsutil::copy_tree(&d, &bak.join(slug)).map_err(|e| tr!("덮어쓰기 전에 백업하지 못했어요 — {e}", "Could not back up before overwriting — {e}"))?;
        }
    }
    let name = &r.target.name;
    fsutil::replace_dir(&ctx.lib.path(slug), &d).map_err(|e| tr!("{name}에 설치하지 못했어요 — {e}", "Could not install to {name} — {e}"))?;
    ctx.man.set(&r.target.id, slug, &lh, fsutil::now_ms());
    Ok(())
}

pub fn uninstall(ctx: &mut Ctx, slug: &str, r: &Resolved) -> Result<(), String> {
    let d = dest(r, slug);
    let name = &r.target.name;
    if d.exists() || std::fs::symlink_metadata(&d).is_ok() {
        fsutil::move_to_trash(&d, ctx.trash, &format!("{}-{slug}", r.target.id))
            .map_err(|e| tr!("{name}에서 지우지 못했어요 — {e}", "Could not remove from {name} — {e}"))?;
    }
    ctx.man.remove(&r.target.id, slug);
    Ok(())
}

/// 설치 위치에서 고친 내용을 라이브러리로 가져와요. 다른 설치 위치는 그대로 둬요.
pub fn pull(ctx: &mut Ctx, slug: &str, r: &Resolved) -> Result<(), String> {
    let d = dest(r, slug);
    if !d.is_dir() {
        return Err(tr!("설치 위치에 그 스킬이 없어요", "That skill is not in this location"));
    }
    ctx.lib.replace_from(slug, &d, ctx.trash)?;
    let h = lib_hash(ctx.lib, slug)?;
    ctx.man.set(&r.target.id, slug, &h, fsutil::now_ms());
    Ok(())
}

/// 설치 위치에만 있는 스킬을 라이브러리로 가져와요. 같은 이름이 라이브러리에 없을 때만 그 이름을 써요.
pub fn import_from_target(ctx: &mut Ctx, dir_name: &str, r: &Resolved) -> Result<String, String> {
    let src = dest(r, dir_name);
    let slug = ctx.lib.import_dir(&src, Some(dir_name))?;
    if slug == dir_name {
        let h = lib_hash(ctx.lib, &slug)?;
        ctx.man.set(&r.target.id, &slug, &h, fsutil::now_ms());
    }
    Ok(slug)
}

/// 설치 위치에만 있는 스킬을 전부 라이브러리로 가져와요. (가져온 스킬, 실패 메시지)
pub fn import_all_external(ctx: &mut Ctx, targets: &[Resolved]) -> (Vec<String>, Vec<String>) {
    let mut slugs = Vec::new();
    let mut errors = Vec::new();
    for r in targets.iter().filter(|r| r.target.enabled) {
        for f in scan(&r.dir) {
            if ctx.lib.exists(&f.dir) {
                continue;
            }
            match import_from_target(ctx, &f.dir, r) {
                Ok(s) => slugs.push(s),
                Err(e) => errors.push(format!("{}: {e}", f.dir)),
            }
        }
    }
    (slugs, errors)
}

/// 업데이트가 있는(설치 위치는 그대로인) 곳만 골라 업데이트해요. 따로 고친 설치본은 건드리지 않아요.
/// 성공한 (스킬, 대상 이름) 목록과 실패 메시지를 돌려줘요.
pub fn push_updates(ctx: &mut Ctx, slugs: &[String], targets: &[Resolved]) -> (Vec<(String, String)>, Vec<String>) {
    let mut done = Vec::new();
    let mut errors = Vec::new();
    for slug in slugs {
        for r in targets.iter().filter(|r| r.target.enabled) {
            match current_status(ctx, slug, r) {
                Ok(Status::LibraryNewer) => match install(ctx, slug, r) {
                    Ok(()) => done.push((slug.clone(), r.target.name.clone())),
                    Err(e) => errors.push(e),
                },
                Ok(_) => {}
                Err(e) => errors.push(e),
            }
        }
    }
    (done, errors)
}

/// 켜진 설치 위치 전부에 한 번에 설치해요. 설치 안 됐거나 업데이트가 있는 곳만 하고,
/// 따로 고친 설치본(확인 필요)은 건드리지 않아요. 성공한 대상 이름과 실패 메시지를 돌려줘요.
pub fn install_everywhere(ctx: &mut Ctx, slug: &str, targets: &[Resolved]) -> (Vec<String>, Vec<String>) {
    let mut done = Vec::new();
    let mut errors = Vec::new();
    for r in targets.iter().filter(|r| r.target.enabled) {
        match current_status(ctx, slug, r) {
            Ok(Status::NotInstalled) | Ok(Status::LibraryNewer) => match install(ctx, slug, r) {
                Ok(()) => done.push(r.target.name.clone()),
                Err(e) => errors.push(e),
            },
            Ok(_) => {}
            Err(e) => {
                errors.push(e);
                break;
            }
        }
    }
    (done, errors)
}

/// 라이브러리에서 이름이 바뀌었을 때: 예전 이름으로 설치된 곳을 새 이름으로 바꿔요.
pub fn follow_rename(ctx: &mut Ctx, old: &str, new: &str, targets: &[Resolved]) -> Vec<String> {
    let mut errors = Vec::new();
    for r in targets {
        let Some(rec) = ctx.man.get(&r.target.id, old).cloned() else { continue };
        let od = dest(r, old);
        let untouched = od.is_dir() && fsutil::hash_dir(&od).ok().as_deref() == Some(rec.hash.as_str());
        ctx.man.remove(&r.target.id, old);
        if !r.target.enabled {
            continue;
        }
        if untouched {
            if let Err(e) = fsutil::move_to_trash(&od, ctx.trash, &format!("{}-{old}-renamed", r.target.id)) {
                let name = &r.target.name;
                errors.push(tr!("{name}: 예전 이름을 지우지 못했어요 — {e}", "{name}: could not remove the old name — {e}"));
                continue;
            }
        }
        if let Err(e) = install(ctx, new, r) {
            errors.push(e);
        }
    }
    errors
}

/// 라이브러리에서 지울 때 같이 지울 설치본(이 앱이 설치했고 따로 고치지 않은 것)을 찾아요.
pub fn removable_installs(ctx: &Ctx, slug: &str, targets: &[Resolved]) -> Vec<Resolved> {
    targets
        .iter()
        .filter(|r| r.target.enabled)
        .filter(|r| {
            let d = dest(r, slug);
            let rec = ctx.man.get(&r.target.id, slug);
            d.is_dir() && rec.is_some() && fsutil::hash_dir(&d).ok().as_deref() == rec.map(|r| r.hash.as_str())
        })
        .cloned()
        .collect()
}

/// 라이브러리에 없는 설치 위치의 스킬 키(`대상id/폴더`) 목록.
pub fn external_keys(lib: &Library, targets: &[Resolved]) -> Vec<(String, String, String)> {
    let slugs = lib.slugs();
    let mut out = Vec::new();
    for r in targets.iter().filter(|r| r.target.enabled) {
        for f in scan(&r.dir) {
            if !slugs.contains(&f.dir) {
                out.push((format!("{}/{}", r.target.id, f.dir), r.target.name.clone(), f.name));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;
    use crate::snapshot::{build, resolve};
    use crate::targets::{builtin_targets, Env};

    struct World {
        t: TestDir,
        lib: Library,
        man: Manifest,
        targets: Vec<Resolved>,
        env: Env,
    }

    fn world() -> World {
        let t = TestDir::new("ops");
        let env = Env { home: t.path().join("home"), codex_home: None };
        let mut ts = builtin_targets();
        ts[2].enabled = true;
        let targets = resolve(&ts, &env);
        let lib = Library::new(t.path().join("lib"));
        lib.create("alpha", "알파 스킬이에요. 알파가 필요할 때 써요.").unwrap();
        World { t, lib, man: Manifest::default(), targets, env }
    }

    impl World {
        fn ctx(&mut self) -> Ctx<'_> {
            Ctx { lib: &self.lib, trash: Box::leak(self.t.path().join("trash").into_boxed_path()), man: &mut self.man }
        }
        fn st(&mut self, slug: &str, i: usize) -> Status {
            let r = self.targets[i].clone();
            current_status(&self.ctx(), slug, &r).unwrap()
        }
    }

    #[test]
    fn install_edit_update_flow() {
        let mut w = world();
        let cc = w.targets[0].clone();
        assert_eq!(w.st("alpha", 0), Status::NotInstalled);
        install(&mut w.ctx(), "alpha", &cc).unwrap();
        assert_eq!(w.st("alpha", 0), Status::Synced);
        assert!(w.env.home.join(".claude/skills/alpha/SKILL.md").is_file());

        // 라이브러리에서 고치면 업데이트가 생겨요.
        w.lib.write_file("alpha", "notes.md", "new").unwrap();
        assert_eq!(w.st("alpha", 0), Status::LibraryNewer);
        let targets = w.targets.clone();
        let (done, errs) = push_updates(&mut w.ctx(), &["alpha".into()], &targets);
        assert!(errs.is_empty());
        assert_eq!(done.len(), 1);
        assert_eq!(w.st("alpha", 0), Status::Synced);

        // 설치 위치에서 고치면 자동 업데이트는 건드리지 않아요.
        std::fs::write(w.env.home.join(".claude/skills/alpha/local.md"), "mine").unwrap();
        assert_eq!(w.st("alpha", 0), Status::TargetModified);
        w.lib.write_file("alpha", "notes.md", "newer").unwrap();
        assert_eq!(w.st("alpha", 0), Status::Conflict);
        let (done, _) = push_updates(&mut w.ctx(), &["alpha".into()], &targets);
        assert!(done.is_empty());

        // 가져오기 하면 라이브러리가 설치본과 같아져요.
        pull(&mut w.ctx(), "alpha", &cc).unwrap();
        assert_eq!(w.st("alpha", 0), Status::Synced);
        assert!(w.lib.path("alpha").join("local.md").is_file());
    }

    #[test]
    fn install_everywhere_skips_modified_copies() {
        let mut w = world();
        let cc = w.targets[0].clone();
        install(&mut w.ctx(), "alpha", &cc).unwrap();
        std::fs::write(w.env.home.join(".claude/skills/alpha/local.md"), "mine").unwrap();
        let targets = w.targets.clone();
        let (done, errs) = install_everywhere(&mut w.ctx(), "alpha", &targets);
        assert!(errs.is_empty());
        // Claude Code는 따로 고쳐서 그대로, 켜진 나머지(Codex, 예전 Codex)는 설치
        assert_eq!(done, vec!["Codex".to_string(), "Codex (예전 위치)".to_string()]);
        assert_eq!(w.st("alpha", 0), Status::TargetModified);
        assert_eq!(w.st("alpha", 1), Status::Synced);
        let (done, _) = install_everywhere(&mut w.ctx(), "alpha", &targets);
        assert!(done.is_empty());
    }

    #[test]
    fn overwrite_backs_up_foreign_content() {
        let mut w = world();
        let cc = w.targets[0].clone();
        let foreign = w.env.home.join(".claude/skills/alpha");
        std::fs::create_dir_all(&foreign).unwrap();
        std::fs::write(foreign.join("SKILL.md"), "---\nname: alpha\ndescription: 남의 것\n---\n").unwrap();
        assert_eq!(w.st("alpha", 0), Status::Foreign);
        install(&mut w.ctx(), "alpha", &cc).unwrap();
        let trash = w.t.path().join("trash");
        let backed_up = std::fs::read_dir(&trash).unwrap().flatten().any(|e| e.path().join("alpha/SKILL.md").is_file());
        assert!(backed_up);
    }

    #[test]
    fn snapshot_counts_and_external() {
        let mut w = world();
        let codex = w.targets[1].clone();
        install(&mut w.ctx(), "alpha", &codex).unwrap();
        let ext = w.env.home.join(".claude/skills/outside");
        std::fs::create_dir_all(&ext).unwrap();
        std::fs::write(ext.join("SKILL.md"), "---\nname: outside\ndescription: 밖에서 만든 스킬\n---\nx").unwrap();
        let (snap, _) = build(&w.lib, &w.targets, &mut w.man, &w.env);
        assert_eq!(snap.counts.skills, 1);
        assert_eq!(snap.counts.installed, 1);
        assert_eq!(snap.counts.external, 1);
        assert_eq!(snap.skills[0].installs["codex"], Status::Synced);
        assert_eq!(snap.skills[0].installs["claude-app"], Status::NotInstalled);
        let cc = &snap.targets[0];
        assert_eq!(cc.path, "~/.claude/skills");
        assert!(!cc.skills[0].in_library);

        let r = w.targets[0].clone();
        let slug = import_from_target(&mut w.ctx(), "outside", &r).unwrap();
        assert_eq!(slug, "outside");
        assert_eq!(w.st("outside", 0), Status::Synced);
        assert!(external_keys(&w.lib, &w.targets).is_empty());
    }

    #[test]
    fn rename_follows_installs() {
        let mut w = world();
        let cc = w.targets[0].clone();
        install(&mut w.ctx(), "alpha", &cc).unwrap();
        w.lib.rename("alpha", "beta").unwrap();
        let targets = w.targets.clone();
        let errs = follow_rename(&mut w.ctx(), "alpha", "beta", &targets);
        assert!(errs.is_empty(), "{errs:?}");
        assert!(!w.env.home.join(".claude/skills/alpha").exists());
        assert!(w.env.home.join(".claude/skills/beta").is_dir());
        assert_eq!(w.st("beta", 0), Status::Synced);
    }

    #[test]
    fn uninstall_moves_to_trash_and_removable_list() {
        let mut w = world();
        let cc = w.targets[0].clone();
        install(&mut w.ctx(), "alpha", &cc).unwrap();
        let targets = w.targets.clone();
        assert_eq!(removable_installs(&w.ctx(), "alpha", &targets).len(), 1);
        uninstall(&mut w.ctx(), "alpha", &cc).unwrap();
        assert!(!w.env.home.join(".claude/skills/alpha").exists());
        assert_eq!(w.st("alpha", 0), Status::NotInstalled);
    }
}
