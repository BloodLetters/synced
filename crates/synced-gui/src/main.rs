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

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([880.0, 560.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("synceD")
            .with_icon(icon_data),
        ..Default::default()
    };

    let initial_url = std::env::args().nth(1);
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
            Ok(Box::new(SyncedApp::new(initial_url, hwnd, cc.egui_ctx.clone(), link_rx.take())))
        }),
    )
}
