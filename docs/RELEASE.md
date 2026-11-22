# 배포와 자동 업데이트

`main`에 푸시할 때마다 `.github/workflows/release.yml`이 버전을 `0.1.<실행번호>`로 올리고,
서명된 Windows 설치 파일(NSIS)·`.sig`·`latest.json`을 GitHub 릴리스에 올려요.
앱은 시작 20초 뒤, 그다음 3시간마다 `latest.json`을 확인해요.

## 처음 한 번만

1. 서명 키를 만들어요. **이 앱 전용으로 새로 만들고, 다른 앱의 키를 재사용하지 마세요.**

   ```bash
   npx tauri signer generate -w ./skills-library.key
   ```

2. GitHub 저장소 → Settings → Secrets and variables → Actions
   - **Secrets** `TAURI_SIGNING_PRIVATE_KEY`: `skills-library.key` 파일 내용
   - **Secrets** `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 키에 암호를 걸었다면 그 암호
   - **Variables** `TAURI_UPDATER_PUBKEY`: `skills-library.key.pub` 파일 내용 (공개키라 비밀이 아니에요)

3. 개인키는 **저장소에 절대 커밋하지 마세요** (`.gitignore`에 `*.key`가 있어요).
   잃어버리면 이미 설치된 앱은 다시는 자동 업데이트를 받을 수 없어요. 안전한 곳에 백업해 두세요.

비밀키가 없으면 워크플로는 경고만 남기고 건너뛰어요.

## 동작

- 빌드할 때 `SKILLSLIB_RELEASE=1`을 넘겨요. 이 값이 있는 빌드만 자동 업데이트와 자동 실행 경로 재등록을 해요.
  개발 빌드(`npm run dev`)가 실수로 업데이트되지 않게 하려는 거예요.
- 자동 업데이트가 켜져 있으면 받아서 설치하고 다시 시작해요. 꺼져 있으면 타이틀바에 배지와
  "설치하고 다시 시작" 버튼을 보여줘요. 설치 직전에 설정과 설치 기록을 저장해요.
- Authenticode 코드 서명 인증서가 없으면 처음 실행할 때 SmartScreen 경고가 떠요. 업데이트 서명과는 별개예요.
