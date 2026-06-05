use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use ndarray::Array4;
use opencv::{core, imgproc, prelude::*, videoio};
use ort::session::Session;
use ort::value::Tensor;

const GESTURE_CLASSES: &[&str] = &[
    "call", "dislike", "fist", "four", "like", "mute", "ok", "one", "palm", "peace",
    "peace_inverted", "rock", "stop", "stop_inverted", "three", "three2", "two_up",
    "two_up_inverted",
];

const MODEL_WIDTH: i32 = 224;
const MODEL_HEIGHT: i32 = 224;
const CONFIDENCE_THRESHOLD: f32 = 0.80;

pub fn run_gesture_loop() -> anyhow::Result<()> {
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?;
    if !cam.is_opened()? {
        anyhow::bail!("Could not open webcam");
    }

    let model_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("gesture.onnx");

    let mut session = Session::builder()?.commit_from_file(model_path)?;
    let mut enigo = Enigo::new(&Settings::default())?;

    let mut last_gesture = String::new();
    let mut gesture_hold_count: u32 = 0;

    loop {
        let mut frame = core::Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
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
            continue;
        }

        let gesture = GESTURE_CLASSES
            .get(gesture_idx)
            .copied()
            .unwrap_or("unknown");

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

    // Normalize [0,255] → [0,1] and convert HWC → CHW
    let pixels: Vec<f32> = data.iter().map(|&b| b as f32 / 255.0).collect();
    let mut chw = vec![0f32; 3 * h * w];
    for row in 0..h {
        for col in 0..w {
            for ch in 0..3 {
                chw[ch * h * w + row * w + col] = pixels[(row * w + col) * 3 + ch];
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
