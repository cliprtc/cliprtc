use once_cell::sync::OnceCell;
use tauri::{Window, WindowEvent};
use tauri_plugin_log::{fern::colors, log, RotationStrategy, Target, TargetKind};

mod clipboard;
mod cmds;
mod global_struct;
mod init;
mod mdns;
mod quic;
mod utils;

pub static APP: OnceCell<tauri::AppHandle> = OnceCell::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    #[cfg(any(dev, debug_assertions))]
                    Target::new(TargetKind::LogDir {
                        file_name: Some("logs.dev".into()),
                    }),
                    #[cfg(not(any(dev, debug_assertions)))]
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .with_colors(colors::ColoredLevelConfig {
                    info: colors::Color::BrightGreen,
                    trace: colors::Color::Cyan,
                    ..colors::ColoredLevelConfig::default()
                })
                .level(log::LevelFilter::Info)
                .max_file_size(50_000 /* bytes */)
                .rotation_strategy(RotationStrategy::KeepAll)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![cmds::exit_app, cmds::restart_app])
        .setup(move |app| {
            APP.get_or_init(|| app.handle().clone());
            init::start();

            #[cfg(any(dev, debug_assertions))]
            open_devtools();

            Ok(())
        })
        .on_window_event(on_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(any(dev, debug_assertions))]
fn open_devtools() {
    use tauri::Manager;
    use utils::constant::SETTINGS_WINDOW_LABEL;

    let app = APP.get().unwrap();
    let settings_window = app.get_webview_window(SETTINGS_WINDOW_LABEL).unwrap();
    settings_window.open_devtools();
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            window.hide().unwrap();
            api.prevent_close();
        }
        _ => {}
    }
}
