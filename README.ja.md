# Skills Library

[English](README.md) | [한국어](README.ko.md) | **日本語** | [简体中文](README.zh-CN.md)

Claude・Claude Code・Codex・Gemini CLI・Grok Build・Antigravity・Cursor・Windsurf の**エージェントスキルを一か所で管理するデスクトップアプリ**です。
スキルの原本をライブラリに置き、必要な場所へインストールし、編集するとインストール先にもすぐ反映されます。

![デモ：スキルを選んで全エージェントに一括インストールし、編集するとすべてのコピーが同期される](docs/screenshots/demo.gif)

## ダウンロード

**[⬇ 最新版をダウンロード](https://github.com/cheapbutsimilar/skills-Library/releases/latest)** — **Windows 10/11（x64）** と **macOS 11 Big Sur 以降（Apple Silicon・Intel）** に対応しています。

| プラットフォーム | ファイル | インストール |
|---|---|---|
| Windows | `…_x64-setup.exe` | インストーラーを実行します |
| macOS | `…_universal.dmg` | 開いて **Skills Library** を **アプリケーション** フォルダーへドラッグします |

インストール後は、どちらのプラットフォームでもアプリが自動で更新されます。インストーラーはこのリポジトリの公開
[リリースワークフロー](.github/workflows/release.yml) でビルドされ、自動更新はすべてアプリの更新署名で検証されます。

> **Windows SmartScreen の警告について** — インストーラーにはまだ有償のコード署名証明書がないため、初回実行時に
> 「Windows によって PC が保護されました」と表示されることがあります。**詳細情報 → 実行** をクリックしてください。

> **macOS Gatekeeper（「開発元が未確認」）について** — まだ Apple の公証（notarization）を受けていないため、初回起動がブロックされることがあります。
> **アプリケーション** フォルダーで **Skills Library を右クリック（または Control + クリック）→「開く」** を選び、ダイアログでもう一度 **「開く」** をクリックしてください。初回のみ必要です。
> 「壊れているため開けません」と表示される場合は、ターミナルでダウンロードの隔離属性を削除してから開き直してください。
>
> ```bash
> xattr -cr "/Applications/Skills Library.app"
> ```

## 機能

- **ライブラリ** — スキル（`SKILL.md` を含むフォルダー）を集め、検索・タグ・お気に入りで整理します。
- **どこへでもインストール** — Claude Code、Codex、Gemini CLI、Grok Build、Antigravity、Cursor、Windsurf、プロジェクトフォルダーに
  インストール・更新・削除できます。1 か所ずつでも、**有効なエージェントすべてにワンクリック**でも可能です。
- **同期** — インストール時に内容のハッシュを記録し、ライブラリとインストール先のどちらが変更されたかを判別します。
  自動同期（既定でオン）では、ライブラリでの編集がすべてのコピーにすぐ反映されます。インストール先で直接編集されたものには触れません。
- **選ぶ前に差分を確認** — インストール先が「変更あり」「両方変更」の状態なら **比較** を開き、ライブラリの原本とインストール先の差分を
  ファイル・行単位で確認して、同じ画面から取り込み／上書きを選べます。
- **GitHub から取り込み** — リポジトリやフォルダーの URL（`https://github.com/owner/repo`、`…/tree/branch/path`）を貼り付けると、
  `SKILL.md` を含むフォルダーをダウンロードして検証し、ライブラリに追加します。フォルダー・ZIP・`.skill` ファイル、ドラッグ＆ドロップにも対応。
- **コードで共有** — スキルの一部またはライブラリ全体を選んで **共有** を押すと、`AB3K9MZ` のような 7 文字のコードとリンクが発行されます。
  別のパソコンでは **取り込み** にコードかリンクを貼り付けるだけで、同じスキルがそのライブラリに入ります。検証と重複処理は ZIP 取り込みと同じです。
  バンドルは 2 MB まで、30 日後に自動で期限切れになります。
- **編集** — フォーム（name・description・本文）または `SKILL.md` の原文で編集でき、スクリプトや参考資料も編集できます。
  ルール（64 文字の小文字・ハイフン名、1024 文字の説明、XML タグ禁止など）は入力中にリアルタイムで検査されます。
- **Claude アプリ** — claude.ai と Claude デスクトップにはスキルフォルダーがないため ZIP でアップロードします。ワンクリックで ZIP を作成し、
  ライブラリが変わると再アップロードを促します。
- **整理** — 同じ名前で内容が異なるスキルは残すバージョンを 1 つ選んで全体を揃え、旧 Codex の場所（`~/.codex/skills`）のスキルは
  現在の場所（`~/.agents/skills`）へ移動します。内容を混ぜることはなく、置き換えられた版はゴミ箱にバックアップされます。
- **安全** — 削除・上書きしたものはすべてゴミ箱フォルダーへ移動するので、元に戻せます。
- **どこからでも検索** — <kbd>Ctrl+Alt+K</kbd>（macOS では <kbd>⌥⌘K</kbd>）でウィンドウを開いてすぐ検索。トレイに常駐し、新しいスキルを通知します。
- **英語・韓国語 UI** — OS の言語に従い、**General › Language** で切り替えられます。

| ライブラリ | 差分比較 |
|---|---|
| ![ライブラリ](docs/screenshots/library.png) | ![差分](docs/screenshots/diff.png) |
| **全エージェントにインストール** | **リンク・コードから取り込み** |
| ![インストール](docs/screenshots/skill-install.png) | ![取り込み](docs/screenshots/github-import.png) |
| **スキルの編集** | **テーマ** |
| ![編集](docs/screenshots/skill-edit.png) | ![テーマ](docs/screenshots/look.png) |

## インストール先

| エージェント | ユーザー全体 | プロジェクト |
|---|---|---|
| Claude Code | `~/.claude/skills` | `<project>/.claude/skills` |
| Codex | `~/.agents/skills` | `<project>/.agents/skills` |
| Codex（旧） | `$CODEX_HOME/skills`（既定 `~/.codex/skills`） | |
| Gemini CLI | `~/.gemini/skills` | `<project>/.gemini/skills` |
| Grok Build | `~/.grok/skills` | `<project>/.grok/skills` |
| Antigravity | `~/.gemini/antigravity/skills` | `<project>/.agent/skills` |
| Cursor | `~/.cursor/skills` | `<project>/.cursor/skills` |
| Windsurf | `~/.codeium/windsurf/skills` | `<project>/.windsurf/skills` |
| Claude アプリ | ZIP アップロード（設定 › 機能 › スキル） | |

各エージェントの設定フォルダーが見つかると、初回起動時に自動で有効になります。パスが異なる場合は アプリの **Locations › Change path** で合わせてください。

## 開発

必要なもの：Rust（stable）、Node 22、Linux の場合は Tauri のシステムライブラリ
（`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`）。

```bash
npm install
```

```bash
npm run preview        # モックデータのブラウザープレビュー → http://127.0.0.1:5199
```

```bash
npm run dev            # 実際のアプリ
```

```bash
npm test               # cargo test --workspace
```

```bash
npm run lint           # clippy + UI チェック
```

詳しい構成やリリース手順は [英語版 README](README.md) と [docs/RELEASE.md](docs/RELEASE.md) を参照してください。

## ライセンス

MIT

エージェントのロゴは [LobeHub Icons](https://github.com/lobehub/lobe-icons)（MIT）のものです。商標は Anthropic、OpenAI、Google、xAI、Anysphere、Cognition に帰属し、どのエージェントかを示す目的でのみ使用しています。

## 姉妹プロジェクト

**[Agent-Meter](https://github.com/cheapbutsimilar/Agent-Meter)** — AI コーディングエージェントの使用量と残量を表示するデスクトップウィジェットです。
