use tauri::Emitter;
use enigo::{Enigo, Settings};

pub fn run_voice_loop(app: tauri::AppHandle) -> anyhow::Result<()> {
    let _enigo = Enigo::new(&Settings::default())?;

    loop {
        // Placeholder: real implementation would use a speech-recognition library.
        std::thread::sleep(std::time::Duration::from_secs(5));
        let command = "play_music";
        let _ = app.emit("voice-command-detected", command);
    }
}
