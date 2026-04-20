// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKit2GTK's drag-drop is broken under Wayland as of 2026-04
    // (tauri/wry tracks it upstream, issue tauri-apps/tauri#11282). Forcing
    // the GTK X11 backend via XWayland makes native drag-drop work
    // reliably with zero visible difference to the user. A user can still
    // opt out by setting GDK_BACKEND explicitly before launch.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("GDK_BACKEND").is_none() {
            // SAFETY: single-threaded process at this point; no other
            // thread can race on environment access.
            unsafe {
                std::env::set_var("GDK_BACKEND", "x11");
            }
        }
    }

    formatlab_lib::run()
}
