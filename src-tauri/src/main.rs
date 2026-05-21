// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKit2GTK EGL_BAD_PARAMETER on some Linux setups
    // (Fedora 40+, Wayland with certain GPU drivers). Must be set before
    // any WebKit init happens.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    book_reader_lib::run()
}
