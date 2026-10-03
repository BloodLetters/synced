#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::ViewportBuilder;
use synced_gui::theme::apply_theme;
use synced_gui::SyncedApp;

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");

    let _enter = rt.enter();

    let icon_data = eframe::egui::IconData {
        rgba: include_bytes!("../../../assets/icon_256.rgba").to_vec(),
        width: 256,
        height: 256,
    };

    let mut initial_url = None;
    let mut start_minimized = false;
    for arg in std::env::args().skip(1) {
        if arg == "--minimized" || arg == "--startup" || arg == "-m" {
            start_minimized = true;
        } else if !arg.starts_with('-') && initial_url.is_none() {
            initial_url = Some(arg);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([880.0, 560.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("synceD")
            .with_icon(icon_data)
            .with_visible(!start_minimized),
        ..Default::default()
    };

    let (link_tx, link_rx) = tokio::sync::mpsc::channel(32);
    synced_gui::server::start_ipc_server(link_tx, 17890);
    let mut link_rx = Some(link_rx);

    eframe::run_native(
        "synceD",
        native_options,
        Box::new(move |cc| {
            use raw_window_handle::HasWindowHandle;
            let hwnd: Option<isize> = cc.window_handle().ok().and_then(|h| {
                if let raw_window_handle::RawWindowHandle::Win32(w) = h.as_raw() {
                    Some(w.hwnd.get())
                } else {
                    None
                }
            });
            let mut style = (*cc.egui_ctx.style()).clone();
            apply_theme(&mut style);
            cc.egui_ctx.set_style(style);
            #[cfg(target_os = "windows")]
            if start_minimized {
                if let Some(h) = hwnd {
                    synced_gui::tray::win32::hide(h);
                }
            }
            Ok(Box::new(SyncedApp::new(initial_url, hwnd, cc.egui_ctx.clone(), link_rx.take())))
        }),
    )
}
