# Skills Library

**English** | [한국어](README.ko.md) | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

A desktop app that keeps all your **agent skills in one place** — for Claude, Claude Code, Codex, Gemini CLI, Grok Build, Antigravity, Cursor and Windsurf.
Keep the originals in a library, install them wherever you need them, and every edit flows to the installed copies.

![Demo: pick a skill, install it to every agent at once, edit it and watch all copies sync](docs/screenshots/demo.gif)

## Download

**[⬇ Download the latest release](https://github.com/cheapbutsimilar/skills-Library/releases/latest)** — supported on **Windows 10/11 (x64)** and **macOS 11 Big Sur or later (Apple Silicon and Intel)**.

| Platform | File | Install |
|---|---|---|
| Windows | `…_x64-setup.exe` | Run the installer |
| macOS | `…_universal.dmg` | Open it and drag **Skills Library** into **Applications** |

Once installed, the app keeps itself up to date on both platforms. Installers are built by the public
[release workflow](.github/workflows/release.yml) and every auto-update is verified with the app's update signature.

> **Windows SmartScreen warning** — the installer is not signed with a paid code-signing certificate yet, so Windows may show
> *“Windows protected your PC”* the first time. Click **More info → Run anyway**.

> **macOS Gatekeeper (“unidentified developer”)** — the app is not notarized by Apple yet, so the first launch may be blocked.
> In **Applications**, **right-click (or Control-click) Skills Library → Open**, then click **Open** in the dialog. You only need to do this once.
> If macOS says the app *“is damaged and can't be opened”*, clear the download quarantine flag in Terminal and open it again:
>
> ```bash
> xattr -cr "/Applications/Skills Library.app"
> ```

## Features

- **Library** — collect skills (folders containing a `SKILL.md`) and organize them with search, tags and favorites.
- **Install everywhere** — install, update or remove a skill for Claude Code, Codex, Gemini CLI, Grok Build, Antigravity,
  Cursor, Windsurf and any project folder — one location at a time, or **all enabled agents in one click**.
- **Sync** — content hashes are recorded at install time, so the app knows whether the library or the installed copy changed.
  With auto sync on (the default), edits in the library spread to every installed copy right away. Copies edited in place are left alone.
- **Diff before you choose** — when an installed copy was *changed in place* or *both sides changed*, open **Compare** to see a
  file-by-file, line-by-line diff between the library original and the installed copy, then import or overwrite from the same view.
- **Import from GitHub** — paste a repository or folder URL (`https://github.com/owner/repo` or `…/tree/branch/path`).
  Every folder with a `SKILL.md` is downloaded, validated and added to the library. Folders, ZIP and `.skill` files work too —
  even by drag and drop. On first launch, skills that are already installed are copied into the library (originals untouched).
- **Edit** — edit with a form (name · description · body) or the raw `SKILL.md`, plus bundled scripts and reference files.
  Rules (64-char lowercase-hyphen names, 1024-char descriptions, no XML tags …) are checked live as you type.
- **Claude app** — claude.ai and Claude desktop have no skills folder, so you upload a ZIP. One click builds it, and the app
  reminds you to re-upload when the library changes.
- **Clean up** — for skills with the same name but different content, pick one version to keep everywhere; skills in the
  legacy Codex location (`~/.codex/skills`) move to the current one (`~/.agents/skills`). Nothing is merged, replaced versions go to the trash.
- **Safe by default** — anything deleted or overwritten is moved to a trash folder, so mistakes can be undone.
- **Search anywhere** — <kbd>Ctrl+Alt+K</kbd> (<kbd>⌥⌘K</kbd> on macOS) opens the window with the search box focused. The app lives in the tray and tells you about new skills.
- **English and Korean** — follows your OS language, switchable under **General › Language**.

| Library | Compare (diff) |
|---|---|
| ![Library](docs/screenshots/library.png) | ![Diff](docs/screenshots/diff.png) |
| **Install to every agent** | **Import from GitHub** |
| ![Install](docs/screenshots/skill-install.png) | ![GitHub import](docs/screenshots/github-import.png) |
| **Edit a skill** | **Locations** |
| ![Edit](docs/screenshots/skill-edit.png) | ![Locations](docs/screenshots/targets.png) |

## Install locations

| Agent | User-wide | Project |
|---|---|---|
| Claude Code | `~/.claude/skills` | `<project>/.claude/skills` |
| Codex | `~/.agents/skills` | `<project>/.agents/skills` |
| Codex (legacy) | `$CODEX_HOME/skills` (default `~/.codex/skills`) — enabled on first launch if the folder exists | |
| Gemini CLI | `~/.gemini/skills` — enabled automatically if `~/.gemini` exists | `<project>/.gemini/skills` |
| Grok Build | `~/.grok/skills` — enabled automatically if `~/.grok` exists | `<project>/.grok/skills` |
| Antigravity | `~/.gemini/antigravity/skills` — enabled automatically if `~/.gemini/antigravity` exists | `<project>/.agent/skills` |
| Cursor | `~/.cursor/skills` — enabled automatically if `~/.cursor` exists | `<project>/.cursor/skills` |
| Windsurf | `~/.codeium/windsurf/skills` — enabled automatically if `~/.codeium/windsurf` exists | `<project>/.windsurf/skills` |
| Claude app | ZIP upload (Settings › Capabilities › Skills) | |

If your paths differ, use **Locations › Change path**. **Add project** lets you pick a project folder and adds the paths above for the agents you choose.

## Sync states

| State | Meaning | What you can do |
|---|---|---|
| Up to date | Same as the library | Remove |
| Update available | The library is newer | Update (automatic with auto sync) |
| Changed in place | Someone edited the installed copy | Compare · Import (installed → library) · Overwrite |
| Both changed | Both sides were edited | Compare · Import · Overwrite (the other side is backed up to the trash) |
| Different skill, same name | Not installed by this app, only the name matches | Compare · Import · Overwrite |

## Development

Requirements: Rust (stable), Node 22, and on Linux the Tauri system libraries
(`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`).

```bash
npm install
```

```bash
npm run preview        # browser preview with mock data and live reload → http://127.0.0.1:5199
```

```bash
npm run dev            # the real app
```

```bash
npm test               # cargo test --workspace
```

```bash
npm run lint           # clippy + UI checks (JS syntax, no inline <style>, every t('…') translated)
```

```bash
npm run screenshots    # docs/screenshots/*.png (English) and docs/screenshots/ko/*.png (Korean)
```

```bash
npm run demo           # docs/screenshots/demo.gif (25 fps, needs ffmpeg)
```

```bash
npm run build          # local installer for this OS
```

```bash
npm run build:mac      # on a Mac: universal .app + .dmg (rustup target add aarch64-apple-darwin x86_64-apple-darwin)
```

UI work happens in the browser preview without Tauri: `scripts/mock-api.js` provides a fake bridge with the same shape as the app.
Append `?lang=en|ko`, `?mode=light`, `?preset=forest`, `?empty=1`, `?welcome=1` or `?update=1` to the preview URL to see other states.

### Translations

UI strings are written as `t('Korean source')` and translated in [`ui/shared/i18n-en.js`](ui/shared/i18n-en.js);
messages from Rust use `tr!("한국어", "English")`. `npm run lint` fails if a string has no English translation.

## Project layout

```
crates/core/        pure logic + unit tests (SKILL.md parsing and checks, library, install/sync, settings, ZIP,
                    GitHub URL import, diff, i18n)
crates/devserver/   browser preview server
src-tauri/src/      app shell: commands, tray, windows, hotkeys, workers, updater, GitHub download
ui/shared/          tokens.css, theme.js, i18n.js + i18n-en.js, util.js, icons.js, logos.js, bridge.js
ui/dashboard/       dashboard (Overview · Library · Skill · Locations · Theme · Alerts & Shortcuts · General)
scripts/            mock-api.js, preview.js, demo-gif.js, check-ui.js, make-icons.py, make-logos.js
```

Settings live in the app data folder — `%APPDATA%\com.cheapbutsimilar.skillslibrary\settings.json` on Windows,
`~/Library/Application Support/com.cheapbutsimilar.skillslibrary/settings.json` on macOS —
and install records in `installs.json` next to it, both written atomically. The library defaults to `library` in the same folder
and can be moved to a Git or cloud-drive folder under **General › Library folder**.

See [docs/RELEASE.md](docs/RELEASE.md) for releases and auto-update.

## License

MIT

Agent logos come from [LobeHub Icons](https://github.com/lobehub/lobe-icons) (MIT) via `scripts/make-logos.js`.
The trademarks belong to Anthropic, OpenAI, Google, xAI, Anysphere and Cognition; they are used only to show which agent is which.

## Sister project

**[Agent-Meter](https://github.com/cheapbutsimilar/Agent-Meter)** — a desktop widget that tracks how much of your AI coding agents' usage you have used and have left.
