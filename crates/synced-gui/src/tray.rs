use eframe::egui;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

#[cfg(target_os = "windows")]
pub mod win32 {
    extern "system" {
        pub fn ShowWindow(hwnd: isize, nCmdShow: i32) -> i32;
        pub fn SetForegroundWindow(hwnd: isize) -> i32;
    }
    pub const SW_HIDE: i32 = 0;
    pub const SW_RESTORE: i32 = 9;

    /// Restores the window and brings it to the foreground.
    pub fn show_and_focus(hwnd: isize) {
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }

    /// Hides the window from screen and taskbar.
    pub fn hide(hwnd: isize) {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// System tray controller for window minimization and background execution.
pub struct TrayManager {
    _tray_icon: TrayIcon,
}

impl TrayManager {
    /// Creates and displays the system tray icon with context menu and OS event handlers.
    pub fn new(hwnd: Option<isize>, ctx: egui::Context) -> Option<Self> {
        let icon_bytes = load_tray_icon_rgba();
        let icon = Icon::from_rgba(icon_bytes, 32, 32).ok()?;

        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Open synceD", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let show_id = show_item.id().clone();
        let quit_id = quit_item.id().clone();

        let _ = tray_menu.append(&show_item);
        let _ = tray_menu.append(&quit_item);

        let ctx_menu = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == quit_id {
                std::process::exit(0);
            } else if event.id == show_id {
                #[cfg(target_os = "windows")]
                if let Some(h) = hwnd {
                    win32::show_and_focus(h);
                }
                ctx_menu.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx_menu.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx_menu.request_repaint();
            }
        }));

        let ctx_click = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            let should_show = match event {
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => true,
                TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => true,
                _ => false,
            };

            if should_show {
                #[cfg(target_os = "windows")]
                if let Some(h) = hwnd {
                    win32::show_and_focus(h);
                }
                ctx_click.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx_click.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx_click.request_repaint();
            }
        }));

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("synceD // Download Manager")
            .with_icon(icon)
            .build()
            .ok()?;

        Some(Self {
            _tray_icon: tray_icon,
        })
    }
}

/// Loads the embedded 32x32 RGBA icon bytes.
fn load_tray_icon_rgba() -> Vec<u8> {
    include_bytes!("../../../assets/icon_32.rgba").to_vec()
}

