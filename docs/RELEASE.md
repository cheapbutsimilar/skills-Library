# 배포와 자동 업데이트

`main`에 푸시할 때마다 `.github/workflows/release.yml`이 버전을 `0.1.<실행번호>`로 올리고,
릴리스 하나에 다음을 올려요.

- Windows: NSIS 설치 파일(`…_x64-setup.exe`)과 `.sig`
- macOS: 유니버설(Apple Silicon + Intel) `…_universal.dmg`, 자동 업데이트용 `.app.tar.gz`와 `.sig`
- 두 플랫폼이 모두 들어간 `latest.json`

순서는 prepare(초안 릴리스) → build(windows-latest, macos-latest를 하나씩) → publish(초안 해제)예요.
빌드를 하나씩 도는 건 두 작업이 `latest.json`을 동시에 고쳐서 한 플랫폼이 빠지는 걸 막으려는 거예요.
번들 형식은 `src-tauri/tauri.windows.conf.json`(nsis)과 `src-tauri/tauri.macos.conf.json`(app, dmg)에 있어요.
앱은 시작 20초 뒤, 그다음 3시간마다 `latest.json`을 확인해요.

## 처음 한 번만

1. 서명 키는 이미 만들어 두었어요. 공개키는 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 들어 있고,
   개인키(`skills-library.key`)는 저장소 밖에서 따로 전달했어요.
   키를 새로 만들어야 한다면 `npx tauri signer generate -w ./skills-library.key`로 만들고,
   공개키를 `tauri.conf.json`에 바꿔 넣으세요 (그러면 기존 설치본은 새 키로 서명된 업데이트를 받지 못해요).

2. GitHub 저장소 → Settings → Secrets and variables → Actions → **New repository secret**
   - 이름 `TAURI_SIGNING_PRIVATE_KEY`, 값: `skills-library.key` 파일 내용 전체
   - 키에 암호를 걸었다면 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`도 넣어요 (지금 키는 암호 없음)

3. 개인키는 **저장소에 절대 커밋하지 마세요** (`.gitignore`에 `*.key`가 있어요).
   잃어버리면 이미 설치된 앱은 다시는 자동 업데이트를 받을 수 없어요. 안전한 곳에 백업해 두세요.

비밀키가 없으면 워크플로는 경고만 남기고 건너뛰어요.

## 동작

- 빌드할 때 `SKILLSLIB_RELEASE=1`을 넘겨요. 이 값이 있는 빌드만 자동 업데이트와 자동 실행 경로 재등록을 해요.
  개발 빌드(`npm run dev`)가 실수로 업데이트되지 않게 하려는 거예요.
- 자동 업데이트가 켜져 있으면 받아서 설치하고 다시 시작해요. 꺼져 있으면 타이틀바에 배지와
  "설치하고 다시 시작" 버튼을 보여줘요. 설치 직전에 설정과 설치 기록을 저장해요.
- Authenticode 코드 서명 인증서가 없으면 처음 실행할 때 SmartScreen 경고가 떠요. 업데이트 서명과는 별개예요.
- macOS 빌드는 ad-hoc 서명(`signingIdentity: "-"`)만 하고 공증(notarization)은 하지 않아요. 그래서 처음 실행할 때
  Gatekeeper가 막을 수 있어요 — 우클릭 → 열기, 또는 `xattr -cr "/Applications/Skills Library.app"`.
  Apple Developer ID가 생기면 `APPLE_CERTIFICATE`·`APPLE_ID` 같은 tauri-action 환경 변수를 더해 서명·공증할 수 있어요.
