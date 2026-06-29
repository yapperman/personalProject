mod agent;
mod gesture;
mod voice;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Err(e) = gesture::run_gesture_loop(handle) {
                    eprintln!("Gesture detection error: {e}");
                }
            });

            #[cfg(target_os = "windows")]
            {
                use tauri::Manager;
                let window = app.get_webview_window("main").unwrap();
                window.with_webview(|wv| {
                    use webview2_com::{
                        Microsoft::Web::WebView2::Win32::*,
                        PermissionRequestedEventHandler,
                    };
                    unsafe {
                        let core = wv.controller().CoreWebView2().unwrap();
                        let mut token = Default::default();
                        core.add_PermissionRequested(
                            &PermissionRequestedEventHandler::create(Box::new(|_, args| {
                                if let Some(args) = args {
                                    let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
                                    args.PermissionKind(&mut kind)?;
                                    if matches!(
                                        kind,
                                        COREWEBVIEW2_PERMISSION_KIND_CAMERA
                                            | COREWEBVIEW2_PERMISSION_KIND_MICROPHONE
                                    ) {
                                        args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
                                    }
                                }
                                Ok(())
                            })),
                            &mut token,
                        )
                        .unwrap();
                    }
                })?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
