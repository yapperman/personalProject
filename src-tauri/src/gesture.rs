use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use ndarray::Array4;
use base64::Engine as _;
use opencv::{core, imgcodecs, imgproc, prelude::*, videoio};
use ort::session::Session;
use ort::value::Tensor;
use tauri::Emitter;

#[derive(serde::Serialize, Clone)]
struct GestureEvent {
    gesture: String,
    confidence: f32,
    detected: bool,
}

const GESTURE_CLASSES: &[&str] = &[
    "call", "dislike", "fist", "four", "grabbing", "grip", "like", "middle_finger", "mute", "no_gesture", "ok", "one", "palm", 
    "peace", "peace_inverted", "rock", "stop", "stop_inverted", "three", "three2", "three3", "two_up", "two_up_inverted"
];

const MODEL_WIDTH: i32 = 224;
const MODEL_HEIGHT: i32 = 224;
const CONFIDENCE_THRESHOLD: f32 = 0.80;

fn open_webcam() -> anyhow::Result<videoio::VideoCapture> {
    // On Windows, CAP_DSHOW is more reliable than CAP_ANY/CAP_MSMF for most webcams.
    // Try DSHOW first, then MSMF, then the generic backend.
    let backends = [videoio::CAP_DSHOW, videoio::CAP_MSMF, videoio::CAP_ANY];
    for &backend in &backends {
        for index in 0..3i32 {
            if let Ok(cam) = videoio::VideoCapture::new(index, backend) {
                if cam.is_opened().unwrap_or(false) {
                    return Ok(cam);
                }
            }
        }
    }
    anyhow::bail!(
        "Could not open webcam. Check that a camera is connected and that \
         Windows Settings > Privacy > Camera has 'Allow desktop apps to access your camera' enabled."
    )
}

pub fn run_gesture_loop(app: tauri::AppHandle) -> anyhow::Result<()> {
    let mut cam = open_webcam()?;

    let model_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("hagrid_vit_gesture.onnx");

    let mut session = Session::builder()?.commit_from_file(model_path)?;
    let mut enigo = Enigo::new(&Settings::default())?;

    let mut last_gesture = String::new();
    let mut gesture_hold_count: u32 = 0;
    let mut frame_count: u64 = 0;

    let jpeg_params = core::Vector::<i32>::from_slice(&[imgcodecs::IMWRITE_JPEG_QUALITY, 60]);

    loop {
        let mut frame = core::Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }

        // Emit a preview frame at ~15fps (every other frame). The frontend displays
        // these instead of calling getUserMedia, avoiding dual-consumer camera conflicts.
        frame_count += 1;
        if frame_count % 2 == 0 {
            let mut preview = core::Mat::default();
            if imgproc::resize(&frame, &mut preview, core::Size::new(240, 180), 0.0, 0.0, imgproc::INTER_LINEAR).is_ok() {
                let mut jpeg_buf = core::Vector::<u8>::new();
                if imgcodecs::imencode(".jpg", &preview, &mut jpeg_buf, &jpeg_params).is_ok() {
                    let data_url = format!(
                        "data:image/jpeg;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(jpeg_buf.as_slice())
                    );
                    let _ = app.emit("camera-frame", data_url);
                }
            }
        }

        let input_tensor = preprocess_frame(&frame)?;
        let outputs = session.run(ort::inputs!["input" => input_tensor])?;
        let (_, scores) = outputs[0].try_extract_tensor::<f32>()?;

        let (gesture_idx, &confidence) = scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap();

        if confidence < CONFIDENCE_THRESHOLD {
            last_gesture.clear();
            gesture_hold_count = 0;
            let _ = app.emit("gesture-detection", GestureEvent {
                gesture: String::new(),
                confidence,
                detected: false,
            });
            continue;
        }

        let gesture = GESTURE_CLASSES
            .get(gesture_idx)
            .copied()
            .unwrap_or("unknown");

        let _ = app.emit("gesture-detection", GestureEvent {
            gesture: gesture.to_string(),
            confidence,
            detected: true,
        });

        if gesture == last_gesture {
            gesture_hold_count += 1;
        } else {
            last_gesture = gesture.to_string();
            gesture_hold_count = 1;
        }

        // Require 8 consecutive frames before triggering (~0.25s at 30fps)
        if gesture_hold_count == 8 {
            trigger_os_action(&mut enigo, gesture)?;
        }
    }
}

fn preprocess_frame(frame: &core::Mat) -> anyhow::Result<Tensor<f32>> {
    let mut resized = core::Mat::default();
    imgproc::resize(
        frame,
        &mut resized,
        core::Size::new(MODEL_WIDTH, MODEL_HEIGHT),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;

    let mut rgb = core::Mat::default();
    imgproc::cvt_color(
        &resized,
        &mut rgb,
        imgproc::COLOR_BGR2RGB,
        0,
        core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    let data = rgb.data_bytes()?;
    let h = MODEL_HEIGHT as usize;
    let w = MODEL_WIDTH as usize;

    // ImageNet normalization: (x/255 - mean) / std, per channel (R, G, B)
    const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
    const STD:  [f32; 3] = [0.229, 0.224, 0.225];

    let mut chw = vec![0f32; 3 * h * w];
    for row in 0..h {
        for col in 0..w {
            for ch in 0..3 {
                let raw = data[(row * w + col) * 3 + ch] as f32 / 255.0;
                chw[ch * h * w + row * w + col] = (raw - MEAN[ch]) / STD[ch];
            }
        }
    }

    let array = Array4::from_shape_vec((1, 3, h, w), chw)?;
    Ok(Tensor::from_array(array)?)
}

fn trigger_os_action(enigo: &mut Enigo, gesture: &str) -> anyhow::Result<()> {
    match gesture {
        "like"          => enigo.key(Key::VolumeUp, Direction::Click)?,
        "dislike"       => enigo.key(Key::VolumeDown, Direction::Click)?,
        "mute"          => enigo.key(Key::VolumeMute, Direction::Click)?,
        "peace"         => enigo.key(Key::RightArrow, Direction::Click)?,
        "peace_inverted"=> enigo.key(Key::LeftArrow, Direction::Click)?,
        "rock"          => enigo.key(Key::MediaNextTrack, Direction::Click)?,
        "fist" => {
            enigo.key(Key::Meta, Direction::Press)?;
            enigo.key(Key::Unicode('d'), Direction::Click)?;
            enigo.key(Key::Meta, Direction::Release)?;
        }
        "palm" | "stop" => {
            enigo.key(Key::Meta, Direction::Press)?;
            enigo.key(Key::Tab, Direction::Click)?;
            enigo.key(Key::Meta, Direction::Release)?;
        }
        "ok" => enigo.button(Button::Left, Direction::Click)?,
        _ => {}
    }
    Ok(())
}
