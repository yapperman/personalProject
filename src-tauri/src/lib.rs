mod agent;
mod gesture;
mod llm;
mod settings;
mod voice;

use llm::{ChatMessage, LlmConfig};
use settings::AppSettings;
use tokio::sync::Mutex;

pub struct AppState {
    pub settings: Mutex<AppSettings>,
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
async fn chat(
    messages: Vec<ChatMessage>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let config = state.settings.lock().await.active.clone();
    llm::chat(&config, &messages)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().await.clone())
}

#[tauri::command]
async fn save_settings(
    new_settings: AppSettings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    settings::save(&new_settings).map_err(|e| e.to_string())?;
    *state.settings.lock().await = new_settings;
    Ok(())
}

// ── App setup ─────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_settings = settings::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            settings: Mutex::new(initial_settings),
        })
        .invoke_handler(tauri::generate_handler![chat, get_settings, save_settings])
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
