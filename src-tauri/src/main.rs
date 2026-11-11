// Windows 桌面应用入口：编译为 GUI 子系统（不弹控制台）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    wudao_notes_lib::run()
}
