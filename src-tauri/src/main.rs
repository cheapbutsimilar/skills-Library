// 릴리스 빌드에서는 Windows 콘솔(PowerShell·cmd) 창을 띄우지 않아요.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    skills_library_lib::run()
}
