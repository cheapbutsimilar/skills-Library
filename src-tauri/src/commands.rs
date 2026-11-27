//! JS → Rust 명령.
//!
//! 창을 만들거나 닫거나, 대화상자를 여는 명령은 Windows WebView2 교착을 피하려고 모두 `async`예요 (HANDOFF §5.1).
//! 무언가 바꾸는 명령은 끝나면 새 스냅샷을 모든 창에 뿌리고, 호출한 쪽에도 돌려줘요.

use skillslib_core::tr;
use crate::state::AppState;
use crate::updater::{self, UpdateInfo};
use crate::{apply, github, hotkeys, menu, windows};
use serde::Serialize;
use serde_json::Value;
use skillslib_core::library::SkillDetail;
use skillslib_core::cleanup;
use skillslib_core::ops;
use skillslib_core::settings::{self, Settings};
use skillslib_core::skill;
use skillslib_core::snapshot::{Resolved, Snapshot};
use skillslib_core::targets::{Agent, CLAUDE_APP};
use skillslib_core::validate::{self, Issue};
use skillslib_core::{diff, fsutil, i18n, zipio};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

type Res<T> = Result<T, String>;

pub fn emit_snapshot(app: &AppHandle) -> Snapshot {
    let st = app.state::<AppState>();
    let snap = st.refresh();
    crate::tray_icon::update(app, &snap.counts);
    let _ = app.emit("snapshot", &snap);
    snap
}

pub fn toast(app: &AppHandle, msg: &str) {
    let _ = app.emit("event", serde_json::json!({ "type": "toast", "msg": msg }));
}

fn find_target(targets: &[Resolved], id: &str) -> Res<Resolved> {
    targets.iter().find(|r| r.target.id == id).cloned().ok_or_else(|| tr!("그 설치 위치를 찾지 못했어요", "Could not find that install location"))
}

/// 자동 동기화가 켜져 있으면, 따로 고치지 않은 설치본을 업데이트해요.
fn auto_sync(st: &AppState, slugs: &[String]) -> (Vec<String>, Vec<String>) {
    if !st.settings().auto_sync {
        return (vec![], vec![]);
    }
    st.with_ops(|ctx, targets, _| {
        let (done, errs) = ops::push_updates(ctx, slugs, targets);
        (done.into_iter().map(|(_, t)| t).collect(), errs)
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitData {
    settings: Settings,
    snapshot: Snapshot,
    version: String,
    platform: &'static str,
    release: bool,
    update: Option<UpdateInfo>,
    data_dir: String,
    default_library: String,
    hotkey_error: Option<String>,
    first_run: bool,
    first_imported: usize,
}

#[tauri::command]
pub async fn init(app: AppHandle, st: State<'_, AppState>) -> Res<InitData> {
    let snapshot = st.lock().snapshot.clone().unwrap_or_else(|| st.refresh());
    Ok(InitData {
        settings: st.settings(),
        snapshot,
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS,
        release: updater::RELEASE,
        update: st.update.lock().unwrap().clone(),
        data_dir: st.paths.data.to_string_lossy().into(),
        default_library: st.paths.default_library.to_string_lossy().into(),
        hotkey_error: st.hotkey_error.lock().unwrap().clone(),
        first_run: st.first_run,
        first_imported: st.first_imported.load(std::sync::atomic::Ordering::SeqCst),
    })
}

#[tauri::command]
pub async fn settings_set(app: AppHandle, st: State<'_, AppState>, patch: Value) -> Res<Settings> {
    let (old, new) = {
        let mut g = st.lock();
        let old = g.settings.clone();
        g.settings = settings::apply_patch(&old, &patch);
        (old, g.settings.clone())
    };
    st.save_settings()?;
    apply::apply(&app, &old, &new);
    Ok(new)
}

#[tauri::command]
pub async fn refresh(app: AppHandle) -> Res<Snapshot> {
    Ok(emit_snapshot(&app))
}

#[tauri::command]
pub async fn skill_get(st: State<'_, AppState>, slug: String) -> Res<SkillDetail> {
    st.library().detail(&slug)
}

/// 편집 중인 내용을 저장하지 않고 검사해요.
#[tauri::command]
pub async fn skill_check(slug: String, raw: String) -> Res<Vec<Issue>> {
    Ok(validate::check(&skill::parse(&raw), &slug, true))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    slug: String,
    synced: Vec<String>,
    errors: Vec<String>,
    snapshot: Snapshot,
}

fn finish_save(app: &AppHandle, st: &AppState, slug: String, renamed_from: Option<String>) -> SaveResult {
    let mut errors = Vec::new();
    if let Some(old) = renamed_from {
        errors.extend(st.with_ops(|ctx, targets, _| ops::follow_rename(ctx, &old, &slug, targets)));
    }
    let (synced, errs) = auto_sync(st, std::slice::from_ref(&slug));
    errors.extend(errs);
    SaveResult { slug, synced, errors, snapshot: emit_snapshot(app) }
}

#[tauri::command]
pub async fn skill_save(app: AppHandle, st: State<'_, AppState>, slug: String, raw: String) -> Res<SaveResult> {
    let (new, old) = st.library().save_raw(&slug, &raw)?;
    Ok(finish_save(&app, &st, new, old))
}

#[tauri::command]
pub async fn skill_save_fields(
    app: AppHandle,
    st: State<'_, AppState>,
    slug: String,
    name: String,
    description: String,
    body: String,
) -> Res<SaveResult> {
    let lib = st.library();
    let cur = lib.detail(&slug)?;
    let raw = skill::with_fields(&cur.raw, &name, &description, Some(&body))?;
    let (new, old) = lib.save_raw(&slug, &raw)?;
    Ok(finish_save(&app, &st, new, old))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlugResult {
    slug: String,
    snapshot: Snapshot,
}

#[tauri::command]
pub async fn skill_create(app: AppHandle, st: State<'_, AppState>, name: String, description: String) -> Res<SlugResult> {
    let slug = st.library().create(&name, &description)?;
    Ok(SlugResult { slug, snapshot: emit_snapshot(&app) })
}

#[tauri::command]
pub async fn skill_duplicate(app: AppHandle, st: State<'_, AppState>, slug: String) -> Res<SlugResult> {
    let slug = st.library().duplicate(&slug)?;
    Ok(SlugResult { slug, snapshot: emit_snapshot(&app) })
}

/// 라이브러리에서 지워요. `uninstall`이면 이 앱이 설치했고 따로 고치지 않은 설치본도 같이 휴지통으로 옮겨요.
#[tauri::command]
pub async fn skill_delete(app: AppHandle, st: State<'_, AppState>, slug: String, uninstall: bool) -> Res<Snapshot> {
    let errs = st.with_ops(|ctx, targets, _| {
        let mut errs = Vec::new();
        if uninstall {
            for r in ops::removable_installs(ctx, &slug, targets) {
                if let Err(e) = ops::uninstall(ctx, &slug, &r) {
                    errs.push(e);
                }
            }
        }
        if let Err(e) = ctx.lib.delete(&slug, ctx.trash) {
            errs.push(e);
        }
        ctx.man.remove(CLAUDE_APP, &slug);
        errs
    });
    let snap = emit_snapshot(&app);
    if let Some(e) = errs.first() {
        return Err(e.clone());
    }
    Ok(snap)
}

#[tauri::command]
pub async fn skill_meta(
    app: AppHandle,
    st: State<'_, AppState>,
    slug: String,
    tags: Option<Vec<String>>,
    favorite: Option<bool>,
) -> Res<Snapshot> {
    st.library().set_meta(&slug, tags, favorite)?;
    Ok(emit_snapshot(&app))
}

#[tauri::command]
pub async fn file_read(st: State<'_, AppState>, slug: String, path: String) -> Res<String> {
    st.library().read_file(&slug, &path)
}

#[tauri::command]
pub async fn file_write(app: AppHandle, st: State<'_, AppState>, slug: String, path: String, content: String) -> Res<SaveResult> {
    st.library().write_file(&slug, &path, &content)?;
    Ok(finish_save(&app, &st, slug, None))
}

#[tauri::command]
pub async fn file_delete(app: AppHandle, st: State<'_, AppState>, slug: String, path: String) -> Res<SaveResult> {
    let trash = st.paths.trash.clone();
    st.library().delete_file(&slug, &path, &trash)?;
    Ok(finish_save(&app, &st, slug, None))
}

#[tauri::command]
pub async fn install(app: AppHandle, st: State<'_, AppState>, slug: String, target: String) -> Res<Snapshot> {
    st.with_ops(|ctx, targets, _| ops::install(ctx, &slug, &find_target(targets, &target)?))?;
    Ok(emit_snapshot(&app))
}

#[tauri::command]
pub async fn uninstall(app: AppHandle, st: State<'_, AppState>, slug: String, target: String) -> Res<Snapshot> {
    st.with_ops(|ctx, targets, _| ops::uninstall(ctx, &slug, &find_target(targets, &target)?))?;
    Ok(emit_snapshot(&app))
}

/// 설치 위치에서 고친 내용을 라이브러리로 가져와요. 자동 동기화가 켜져 있으면 다른 곳에도 퍼져요.
#[tauri::command]
pub async fn pull(app: AppHandle, st: State<'_, AppState>, slug: String, target: String) -> Res<SaveResult> {
    st.with_ops(|ctx, targets, _| ops::pull(ctx, &slug, &find_target(targets, &target)?))?;
    Ok(finish_save(&app, &st, slug, None))
}

#[tauri::command]
pub async fn import_target(app: AppHandle, st: State<'_, AppState>, target: String, dir: String) -> Res<SlugResult> {
    let slug = st.with_ops(|ctx, targets, _| ops::import_from_target(ctx, &dir, &find_target(targets, &target)?))?;
    Ok(SlugResult { slug, snapshot: emit_snapshot(&app) })
}

/// 설치 위치에만 있는 스킬을 전부 라이브러리로 가져와요.
#[tauri::command]
pub async fn import_all_external(app: AppHandle) -> Res<ImportResult> {
    Ok(import_all_external_inner(&app))
}

pub fn import_all_external_inner(app: &AppHandle) -> ImportResult {
    let st = app.state::<AppState>();
    let (slugs, errors) = st.with_ops(|ctx, targets, _| ops::import_all_external(ctx, targets));
    ImportResult { slugs, errors, snapshot: emit_snapshot(app) }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub done: usize,
    pub errors: Vec<String>,
    snapshot: Snapshot,
}

#[tauri::command]
pub async fn sync_all(app: AppHandle, st: State<'_, AppState>) -> Res<SyncResult> {
    Ok(sync_all_inner(&app, &st))
}

pub fn sync_all_inner(app: &AppHandle, st: &AppState) -> SyncResult {
    let (done, errors) = st.with_ops(|ctx, targets, _| {
        let slugs = ctx.lib.slugs();
        ops::push_updates(ctx, &slugs, targets)
    });
    SyncResult { done: done.len(), errors, snapshot: emit_snapshot(app) }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub slugs: Vec<String>,
    pub errors: Vec<String>,
    snapshot: Snapshot,
}

fn import_roots(lib: &skillslib_core::library::Library, roots: &[PathBuf], label: &str) -> (Vec<String>, Vec<String>) {
    let mut slugs = Vec::new();
    let mut errors = Vec::new();
    for r in roots {
        match lib.import_dir(r, None) {
            Ok(s) => slugs.push(s),
            Err(e) => errors.push(format!("{label}: {e}")),
        }
    }
    (slugs, errors)
}

/// GitHub 저장소·하위 폴더 URL에서 스킬을 받아 검사하고 라이브러리에 넣어요.
/// 오류가 있는 스킬은 건너뛰고 이유를 `errors`에 담아요.
#[tauri::command]
pub async fn import_github(app: AppHandle, url: String) -> Res<ImportResult> {
    let st = app.state::<AppState>();
    let tmp = st.paths.tmp.clone();
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let f = github::fetch(&url, &tmp).await?;
    let cleanup = || {
        if let Some(d) = f.root.parent() {
            let _ = std::fs::remove_dir_all(d);
        }
    };
    let (roots, skipped) = skillslib_core::github::validated_roots(&f.root);
    if roots.is_empty() {
        cleanup();
        return Err(match skipped.first() {
            Some(e) => tr!("가져올 수 있는 스킬이 없어요 — {e}", "No importable skills — {e}"),
            None => {
                let l = &f.label;
                tr!("{l}에서 SKILL.md가 있는 폴더를 찾지 못했어요", "No folder with a SKILL.md was found in {l}")
            }
        });
    }
    let (slugs, mut errors) = import_roots(&st.library(), &roots, &f.label);
    errors.extend(skipped);
    cleanup();
    Ok(ImportResult { slugs, errors, snapshot: emit_snapshot(&app) })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareResult {
    code: String,
    url: String,
    expires_in_days: u32,
    bytes: u64,
    count: usize,
}

/// 고른 스킬(목록이 비어 있으면 라이브러리 전체)을 ZIP 하나로 묶어 공유 코드·링크를 만들어요.
/// 코드는 30일 뒤에 자동으로 만료돼요.
#[tauri::command]
pub async fn share_upload(st: State<'_, AppState>, slugs: Vec<String>) -> Res<ShareResult> {
    let lib = st.library();
    let picked = if slugs.is_empty() { lib.slugs() } else { slugs };
    if picked.is_empty() {
        return Err(tr!(
            "공유할 스킬이 없어요 — 먼저 라이브러리에 스킬을 넣어 주세요",
            "There is nothing to share — add skills to the library first"
        ));
    }
    let mut pairs = Vec::with_capacity(picked.len());
    for slug in picked {
        if !fsutil::safe_rel(&slug) || slug.contains(['/', '\\']) {
            return Err(tr!("스킬 이름이 올바르지 않아요", "The skill name is not valid"));
        }
        if !lib.exists(&slug) {
            return Err(tr!("라이브러리에 {slug}가 없어요", "There is no {slug} in the library"));
        }
        pairs.push((slug.clone(), lib.path(&slug)));
    }
    std::fs::create_dir_all(&st.paths.tmp).map_err(|e| e.to_string())?;
    let zip = st.paths.tmp.join(format!("share-{}.zip", fsutil::now_ms()));
    let bundle = skillslib_core::share::pack(&pairs, &zip)?;
    let data = std::fs::read(&zip).map_err(|e| tr!("ZIP을 읽지 못했어요 — {e}", "Could not read the ZIP — {e}"))?;
    let _ = std::fs::remove_file(&zip);
    let p = crate::share::upload(data).await?;
    Ok(ShareResult { code: p.code, url: p.url, expires_in_days: p.expires_in_days, bytes: bundle.bytes, count: bundle.count })
}

/// 공유 코드·링크로 받은 묶음을 풀어 라이브러리에 넣어요 (ZIP 가져오기와 같은 검사·중복 확인).
#[tauri::command]
pub async fn import_share(app: AppHandle, input: String) -> Res<ImportResult> {
    let code = skillslib_core::share::parse_code(&input)?;
    let data = crate::share::download(&code).await?;
    let st = app.state::<AppState>();
    let dir = st.paths.tmp.join(format!("share-{}", fsutil::now_ms()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let zip = dir.join(format!("skills-{code}.zip"));
    std::fs::write(&zip, &data).map_err(|e| tr!("받은 파일을 저장하지 못했어요 — {e}", "Could not save the download — {e}"))?;
    let r = import_paths_inner(&app, &[zip]);
    let _ = std::fs::remove_dir_all(&dir);
    if r.slugs.is_empty() && r.errors.is_empty() {
        return Err(tr!(
            "{code} 묶음에서 가져올 스킬을 찾지 못했어요",
            "No importable skill was found in bundle {code}"
        ));
    }
    Ok(r)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    slug: String,
    target: String,
    target_name: String,
    library_path: String,
    installed_path: String,
    diff: diff::DirDiff,
}

/// 라이브러리 원본과 설치본의 차이를 보여줘요 (덮어쓰기·가져오기 전에).
#[tauri::command]
pub async fn diff(st: State<'_, AppState>, slug: String, target: String) -> Res<DiffResult> {
    if slug.is_empty() || !fsutil::safe_rel(&slug) || slug.contains(['/', '\\']) {
        return Err(tr!("스킬 이름이 올바르지 않아요", "The skill name is not valid"));
    }
    let resolved = st.resolved(&st.settings());
    let r = find_target(&resolved, &target)?;
    let lib = st.library();
    if !lib.exists(&slug) {
        return Err(tr!("라이브러리에 그 스킬이 없어요", "That skill is not in the library"));
    }
    let inst = r.dir.join(&slug);
    if !inst.is_dir() {
        return Err(tr!("설치 위치에 그 스킬이 없어요", "That skill is not in this location"));
    }
    let lp = lib.path(&slug);
    Ok(DiffResult {
        diff: diff::compare(&lp, &inst),
        library_path: lp.to_string_lossy().into(),
        installed_path: inst.to_string_lossy().into(),
        target_name: r.target.name.clone(),
        target,
        slug,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallAllResult {
    done: Vec<String>,
    errors: Vec<String>,
    snapshot: Snapshot,
}

/// 켜진 설치 위치 전부에 한 번에 설치·업데이트해요. 따로 고친 설치본은 그대로 둬요.
#[tauri::command]
pub async fn install_all(app: AppHandle, st: State<'_, AppState>, slug: String) -> Res<InstallAllResult> {
    let (done, errors) = st.with_ops(|ctx, targets, _| ops::install_everywhere(ctx, &slug, targets));
    Ok(InstallAllResult { done, errors, snapshot: emit_snapshot(&app) })
}

/// 화면 언어가 정해지거나 바뀌면 불러요. 앱 메시지·트레이 메뉴도 그 언어로 바꿔요.
#[tauri::command]
pub async fn set_lang(app: AppHandle, lang: String) -> Res<Snapshot> {
    i18n::set_lang(&lang);
    menu::relabel(&app);
    Ok(emit_snapshot(&app))
}

/// 폴더·ZIP·SKILL.md 경로들을 라이브러리로 가져와요 (대화상자·끌어다 놓기 공통).
pub fn import_paths_inner(app: &AppHandle, paths: &[PathBuf]) -> ImportResult {
    let st = app.state::<AppState>();
    let lib = st.library();
    let mut slugs = Vec::new();
    let mut errors = Vec::new();
    for p in paths {
        let label = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let is_zip = p.is_file()
            && p.extension().map(|e| e.to_string_lossy().to_lowercase()).is_some_and(|e| e == "zip" || e == "skill");
        let roots: Result<(Vec<PathBuf>, Option<PathBuf>), String> = if is_zip {
            let tmp = st.paths.tmp.join(format!("zip-{}", fsutil::now_ms()));
            zipio::extract(p, &tmp).map(|r| (r, Some(tmp)))
        } else if p.is_file() && label.eq_ignore_ascii_case("SKILL.md") {
            Ok((p.parent().map(|d| vec![d.to_path_buf()]).unwrap_or_default(), None))
        } else if p.is_dir() {
            let r = zipio::find_skill_roots(p, 2);
            if r.is_empty() {
                Err(tr!("SKILL.md가 있는 폴더를 찾지 못했어요", "No folder with a SKILL.md was found"))
            } else {
                Ok((r, None))
            }
        } else {
            Err(tr!("폴더, ZIP, .skill, SKILL.md만 가져올 수 있어요", "Only folders, ZIP, .skill and SKILL.md can be imported"))
        };
        match roots {
            Ok((roots, tmp)) => {
                let (s, e) = import_roots(&lib, &roots, &label);
                slugs.extend(s);
                errors.extend(e);
                if let Some(t) = tmp {
                    let _ = std::fs::remove_dir_all(t);
                }
            }
            Err(e) => errors.push(format!("{label}: {e}")),
        }
    }
    ImportResult { slugs, errors, snapshot: emit_snapshot(app) }
}

#[tauri::command]
pub async fn import_pick(app: AppHandle, kind: String) -> Res<Option<ImportResult>> {
    let d = app.dialog().file().set_title(tr!("스킬 가져오기", "Import skills"));
    let picked: Option<Vec<PathBuf>> = if kind == "zip" {
        d.add_filter(tr!("스킬 ZIP", "Skill ZIP"), &["zip", "skill"])
            .blocking_pick_files()
            .map(|v| v.into_iter().filter_map(|f| f.into_path().ok()).collect())
    } else {
        d.blocking_pick_folder().and_then(|f| f.into_path().ok()).map(|p| vec![p])
    };
    Ok(picked.map(|paths| import_paths_inner(&app, &paths)))
}

#[tauri::command]
pub async fn import_paths(app: AppHandle, paths: Vec<String>) -> Res<ImportResult> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    Ok(import_paths_inner(&app, &paths))
}

/// Claude 앱에 올릴 ZIP을 만들어요. 저장 위치를 물어보고, 내보낸 내용을 기록해요.
#[tauri::command]
pub async fn export_zip(app: AppHandle, st: State<'_, AppState>, slug: String) -> Res<Option<String>> {
    let lib = st.library();
    if !lib.exists(&slug) {
        return Err(tr!("라이브러리에 그 스킬이 없어요", "That skill is not in the library"));
    }
    let mut d = app.dialog().file().set_title(tr!("Claude 앱용 ZIP 저장", "Save ZIP for the Claude app")).set_file_name(format!("{slug}.zip")).add_filter("ZIP", &["zip"]);
    if let Ok(dl) = app.path().download_dir() {
        d = d.set_directory(dl);
    }
    let Some(out) = d.blocking_save_file().and_then(|f| f.into_path().ok()) else {
        return Ok(None);
    };
    let out = if out.extension().is_none() { out.with_extension("zip") } else { out };
    zipio::export(&lib.path(&slug), &slug, &out).map_err(|e| tr!("ZIP을 만들지 못했어요 — {e}", "Could not create the ZIP — {e}"))?;
    let hash = fsutil::hash_dir(&lib.path(&slug)).unwrap_or_default();
    {
        let mut g = st.lock();
        g.manifest.set(CLAUDE_APP, &slug, &hash, fsutil::now_ms());
        st.save_manifest_locked(&g);
    }
    emit_snapshot(&app);
    let _ = app.opener().reveal_item_in_dir(&out);
    Ok(Some(out.to_string_lossy().into()))
}

fn pick_folder(app: &AppHandle, title: &str) -> Option<PathBuf> {
    app.dialog().file().set_title(title).blocking_pick_folder().and_then(|f| f.into_path().ok())
}

fn folder_label(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string_lossy().into())
}

/// 정리 미리보기: 무엇을 맞추고 옮길지 보여줘요. 아무것도 바꾸지 않아요.
#[tauri::command]
pub async fn cleanup_plan(st: State<'_, AppState>) -> Res<cleanup::Plan> {
    Ok(st.with_ops(|ctx, targets, _| cleanup::plan(ctx.lib, targets, ctx.man, &st.env)))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    report: cleanup::Report,
    settings: Settings,
    snapshot: Snapshot,
}

/// 정리 실행. 예전 위치가 비면 그 설치 위치는 꺼요 (다시 켤 수 있어요).
#[tauri::command]
pub async fn cleanup_apply(app: AppHandle, st: State<'_, AppState>, choices: cleanup::Choices) -> Res<CleanupResult> {
    let report = st.with_ops(|ctx, targets, _| cleanup::apply(ctx, targets, &choices));
    if choices.legacy && report.legacy_emptied {
        let (old, new) = {
            let mut g = st.lock();
            let old = g.settings.clone();
            if let Some(t) = g.settings.targets.iter_mut().find(|t| t.id == cleanup::LEGACY_FROM) {
                t.enabled = false;
            }
            (old, g.settings.clone())
        };
        st.save_settings()?;
        apply::apply(&app, &old, &new);
    }
    Ok(CleanupResult { report, settings: st.settings(), snapshot: emit_snapshot(&app) })
}

/// 프로젝트 폴더를 골라 설치 위치로 추가해요.
/// agents: 쉼표로 나눈 claude-code, codex, gemini, grok, antigravity, cursor, windsurf (예전 값 "both"는 Claude Code + Codex)
#[tauri::command]
pub async fn target_add_project(app: AppHandle, st: State<'_, AppState>, agent: String) -> Res<Option<Settings>> {
    let mut agents: Vec<Agent> = Vec::new();
    for a in agent.split(',').map(str::trim) {
        if a == "both" {
            agents.extend([Agent::ClaudeCode, Agent::Codex]);
        } else if let Some(x) = Agent::from_id(a) {
            agents.push(x);
        }
    }
    if agents.is_empty() {
        return Err(tr!("에이전트를 하나 이상 골라 주세요", "Pick at least one agent"));
    }
    let Some(dir) = pick_folder(&app, &tr!("프로젝트 폴더 고르기", "Choose a project folder")) else { return Ok(None) };
    let label = folder_label(&dir);
    let (old, new, added) = {
        let mut g = st.lock();
        let old = g.settings.clone();
        let mut added = 0;
        for a in agents {
            let path = dir.join(a.project_subdir());
            let name = format!("{label} · {}", a.label());
            if g.settings.add_target(&name, a, &path.to_string_lossy()).is_some() {
                added += 1;
            }
        }
        (old, g.settings.clone(), added)
    };
    if added == 0 {
        return Err(tr!("이미 추가된 프로젝트예요", "That project is already added"));
    }
    st.save_settings()?;
    apply::apply(&app, &old, &new);
    Ok(Some(new))
}

/// 설치 위치 폴더를 바꿔요.
#[tauri::command]
pub async fn target_pick_path(app: AppHandle, st: State<'_, AppState>, id: String) -> Res<Option<Settings>> {
    let Some(dir) = pick_folder(&app, &tr!("스킬 폴더 고르기", "Choose a skills folder")) else { return Ok(None) };
    let (old, new) = {
        let mut g = st.lock();
        let old = g.settings.clone();
        let mut s = old.clone();
        let t = s.targets.iter_mut().find(|t| t.id == id).ok_or_else(|| tr!("그 설치 위치를 찾지 못했어요", "Could not find that install location"))?;
        t.path = dir.to_string_lossy().into();
        s.sanitize();
        g.manifest.remove_target(&id);
        g.settings = s;
        st.save_manifest_locked(&g);
        (old, g.settings.clone())
    };
    st.save_settings()?;
    apply::apply(&app, &old, &new);
    Ok(Some(new))
}

/// 기본 경로로 되돌려요 (기본 대상만).
#[tauri::command]
pub async fn target_reset_path(app: AppHandle, st: State<'_, AppState>, id: String) -> Res<Settings> {
    let def = skillslib_core::targets::builtin_targets().into_iter().find(|t| t.id == id).ok_or_else(|| tr!("기본 설치 위치가 아니에요", "Not a built-in install location"))?;
    let (old, new) = {
        let mut g = st.lock();
        let old = g.settings.clone();
        if let Some(t) = g.settings.targets.iter_mut().find(|t| t.id == id) {
            t.path = def.path;
        }
        g.manifest.remove_target(&id);
        st.save_manifest_locked(&g);
        (old, g.settings.clone())
    };
    st.save_settings()?;
    apply::apply(&app, &old, &new);
    Ok(new)
}

/// 설치 위치를 목록에서 빼요. 폴더 안의 스킬은 그대로 둬요.
#[tauri::command]
pub async fn target_remove(app: AppHandle, st: State<'_, AppState>, id: String) -> Res<Settings> {
    let (old, new) = {
        let mut g = st.lock();
        let old = g.settings.clone();
        if old.target(&id).is_some_and(|t| t.builtin) {
            return Err(tr!("기본 설치 위치는 끌 수만 있어요", "Built-in install locations can only be turned off"));
        }
        g.settings.targets.retain(|t| t.id != id);
        g.manifest.remove_target(&id);
        st.save_manifest_locked(&g);
        (old, g.settings.clone())
    };
    st.save_settings()?;
    apply::apply(&app, &old, &new);
    Ok(new)
}

/// 라이브러리 폴더를 바꿔요. 새 폴더가 비어 있으면 지금 스킬을 복사해 가요.
#[tauri::command]
pub async fn library_change_dir(app: AppHandle, st: State<'_, AppState>) -> Res<Option<Settings>> {
    let Some(dir) = pick_folder(&app, &tr!("라이브러리 폴더 고르기", "Choose a library folder")) else { return Ok(None) };
    let cur = st.library_dir(&st.settings());
    if dir == cur {
        return Ok(Some(st.settings()));
    }
    if dir.starts_with(&cur) {
        return Err(tr!("지금 라이브러리 폴더 안쪽은 고를 수 없어요", "You cannot pick a folder inside the current library folder"));
    }
    let empty = std::fs::read_dir(&dir).map(|mut r| r.next().is_none()).unwrap_or(true);
    if empty && cur.is_dir() {
        fsutil::copy_tree(&cur, &dir).map_err(|e| tr!("스킬을 새 폴더로 복사하지 못했어요 — {e}", "Could not copy skills to the new folder — {e}"))?;
    }
    settings_set(app.clone(), st, serde_json::json!({ "libraryDir": dir.to_string_lossy() })).await.map(Some)
}

#[tauri::command]
pub async fn open_path(app: AppHandle, path: String) -> Res<()> {
    let p = PathBuf::from(&path);
    if p.is_dir() {
        app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
    } else if p.exists() {
        app.opener().reveal_item_in_dir(&p).map_err(|e| e.to_string())
    } else if let Some(parent) = p.ancestors().find(|a| a.is_dir()) {
        app.opener().open_path(parent.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
    } else {
        Err(tr!("폴더가 아직 없어요", "The folder does not exist yet"))
    }
}

#[tauri::command]
pub async fn open_skill_dir(app: AppHandle, st: State<'_, AppState>, slug: String) -> Res<()> {
    let p = st.library().path(&slug);
    open_path(app, p.to_string_lossy().into()).await
}

#[tauri::command]
pub async fn open_special(app: AppHandle, st: State<'_, AppState>, which: String) -> Res<()> {
    let p = match which.as_str() {
        "data" => st.paths.data.clone(),
        "trash" => st.paths.trash.clone(),
        _ => st.library_dir(&st.settings()),
    };
    let _ = std::fs::create_dir_all(&p);
    open_path(app, p.to_string_lossy().into()).await
}

#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> Res<()> {
    if !url.starts_with("https://") {
        return Err(tr!("https 주소만 열 수 있어요", "Only https URLs can be opened"));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn win_minimize(app: AppHandle) -> Res<()> {
    if let Some(w) = app.get_webview_window(windows::MAIN) {
        w.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn win_toggle_maximize(app: AppHandle) -> Res<()> {
    if let Some(w) = app.get_webview_window(windows::MAIN) {
        if w.is_maximized().unwrap_or(false) {
            w.unmaximize().map_err(|e| e.to_string())?;
        } else {
            w.maximize().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn win_close(app: AppHandle) -> Res<()> {
    if let Some(w) = app.get_webview_window(windows::MAIN) {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 단축키 녹음 시작: 전역 단축키를 모두 풀어요.
#[tauri::command]
pub async fn hotkey_suspend(app: AppHandle) -> Res<()> {
    hotkeys::suspend(&app);
    Ok(())
}

/// 단축키 녹음 끝(취소 포함): 지금 설정으로 다시 등록해요.
#[tauri::command]
pub async fn hotkey_resume(app: AppHandle) -> Res<Option<String>> {
    Ok(hotkeys::resume(&app))
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Res<Option<UpdateInfo>> {
    updater::check(&app).await
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> Res<()> {
    updater::install(&app).await
}
