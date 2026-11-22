fn main() {
    // 릴리스 워크플로가 SKILLSLIB_RELEASE=1을 넘겨요. 이 값이 있는 빌드만 자동 업데이트·자동 실행 재등록을 해요.
    println!("cargo:rerun-if-env-changed=SKILLSLIB_RELEASE");
    tauri_build::build()
}
