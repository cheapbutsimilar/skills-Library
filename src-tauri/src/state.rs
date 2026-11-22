//! AppState: 설정·설치 기록·스냅샷을 Mutex 하나로 들고 있어요. 저장은 모두 원자적으로 해요.

use crate::updater::UpdateInfo;
use skillslib_core::library::Library;
use skillslib_core::ops::Ctx;
use skillslib_core::settings::{self, Settings};
use skillslib_core::snapshot::{self, Resolved, Snapshot};
use skillslib_core::sync::Manifest;
use skillslib_core::targets::{self, Env};
use skillslib_core::{atomic, fsutil};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Mutex, MutexGuard};

pub struct Paths {
    pub data: PathBuf,
    pub settings: PathBuf,
    pub manifest: PathBuf,
    pub trash: PathBuf,
    pub tmp: PathBuf,
    pub default_library: PathBuf,
}

pub struct Inner {
    pub settings: Settings,
    pub manifest: Manifest,
    pub snapshot: Option<Snapshot>,
    pub fingerprint: u64,
    /// 창 위치가 바뀌었는데 아직 저장하지 않았어요 (유지 스레드가 모아서 저장해요).
    pub win_dirty: bool,
}

pub struct AppState {
    pub paths: Paths,
    pub env: Env,
    pub inner: Mutex<Inner>,
    pub hotkeys_suspended: AtomicBool,
    pub hotkey_error: Mutex<Option<String>>,
    pub update: Mutex<Option<UpdateInfo>>,
    pub first_run: bool,
    /// 첫 실행 때 설치 위치에서 가져온 스킬 수 (환영 안내용)
    pub first_imported: AtomicUsize,
}

impl AppState {
    pub fn load(data: PathBuf, home: PathBuf) -> Self {
        let paths = Paths {
            settings: data.join("settings.json"),
            manifest: data.join("installs.json"),
            trash: data.join("trash"),
            tmp: data.join("tmp"),
            default_library: data.join("library"),
            data,
        };
        let env = Env::from_os(home);
        let first_run = !paths.settings.exists() && !atomic::backup_path(&paths.settings).exists();
        let mut s = atomic::read_json_value(&paths.settings).map(settings::from_value).unwrap_or_default();
        if first_run {
            // 예전 Codex 위치에 스킬 폴더가 있으면 그것도 켜요.
            if let Some(t) = s.targets.iter_mut().find(|t| t.id == "codex-legacy") {
                t.enabled = targets::expand(&t.path, &env).is_dir();
            }
        }
        let manifest: Manifest = atomic::read_json(&paths.manifest).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&paths.tmp);
        let st = AppState {
            paths,
            env,
            inner: Mutex::new(Inner { settings: s, manifest, snapshot: None, fingerprint: 0, win_dirty: false }),
            hotkeys_suspended: AtomicBool::new(false),
            hotkey_error: Mutex::new(None),
            update: Mutex::new(None),
            first_run,
            first_imported: AtomicUsize::new(0),
        };
        if first_run {
            let _ = st.save_settings();
        }
        st
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn settings(&self) -> Settings {
        self.lock().settings.clone()
    }

    pub fn library_dir(&self, s: &Settings) -> PathBuf {
        if s.library_dir.is_empty() {
            self.paths.default_library.clone()
        } else {
            targets::expand(&s.library_dir, &self.env)
        }
    }

    pub fn library(&self) -> Library {
        Library::new(self.library_dir(&self.settings()))
    }

    pub fn resolved(&self, s: &Settings) -> Vec<Resolved> {
        snapshot::resolve(&s.targets, &self.env)
    }

    /// 변경 감지할 폴더: 라이브러리 + 켜진 설치 위치
    pub fn roots(&self, s: &Settings) -> Vec<PathBuf> {
        let mut v = vec![self.library_dir(s)];
        v.extend(self.resolved(s).into_iter().filter(|r| r.target.enabled).map(|r| r.dir));
        v
    }

    pub fn save_settings(&self) -> Result<(), String> {
        let s = self.settings();
        atomic::write_json(&self.paths.settings, &s).map_err(|e| format!("설정을 저장하지 못했어요 — {e}"))
    }

    pub fn save_manifest_locked(&self, inner: &Inner) {
        if let Err(e) = atomic::write_json(&self.paths.manifest, &inner.manifest) {
            eprintln!("installs.json 저장 실패: {e}");
        }
    }

    pub fn save_all(&self) {
        let _ = self.save_settings();
        let g = self.lock();
        self.save_manifest_locked(&g);
    }

    /// 스냅샷을 새로 만들어요.
    pub fn refresh(&self) -> Snapshot {
        let mut g = self.lock();
        let s = g.settings.clone();
        let lib = Library::new(self.library_dir(&s));
        let _ = std::fs::create_dir_all(&lib.dir);
        let resolved = self.resolved(&s);
        let (snap, changed) = snapshot::build(&lib, &resolved, &mut g.manifest, &self.env);
        if changed {
            self.save_manifest_locked(&g);
        }
        g.fingerprint = fsutil::fingerprint(&self.roots(&s));
        g.snapshot = Some(snap.clone());
        snap
    }

    /// 라이브러리·설치 기록을 다루는 동작을 실행하고 기록을 저장해요.
    pub fn with_ops<R>(&self, f: impl FnOnce(&mut Ctx, &[Resolved], &Settings) -> R) -> R {
        let mut g = self.lock();
        let s = g.settings.clone();
        let lib = Library::new(self.library_dir(&s));
        let resolved = self.resolved(&s);
        let trash = self.paths.trash.clone();
        let r = {
            let mut ctx = Ctx { lib: &lib, trash: &trash, man: &mut g.manifest };
            f(&mut ctx, &resolved, &s)
        };
        self.save_manifest_locked(&g);
        r
    }
}
