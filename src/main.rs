use nannou::prelude::*;
use nannou_audio as audio;
use nannou_audio::Buffer;
use std::f64::consts::PI;
use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::thread;

fn main() {
    nannou::app(model).update(update).run();
}

struct PulseData {
    min: f32,
    max: f32,
    raw: f32,
}

// More sophisticated audio state
struct Audio {
    phase: f64,
    hz: f64,
    target_hz: f64,
    amplitude: f32,
    target_amplitude: f32,
    // Add a simple smoothing factor (Liner Interpolation)
    lerp_factor: f32,
}

struct Model {
    pulse: f32, 
    stream: audio::Stream<Audio>,
    receiver: mpsc::Receiver<PulseData>,
    history: Vec<f32>,
    beat_detected: bool,
}

fn model(app: &App) -> Model {
    app.new_window().size(800, 800).view(view).build().unwrap();

    let (tx, rx) = mpsc::channel();
    // Adjust port for your OS
    let port_name = "/dev/cu.usbmodemDC5475C4CF702"; 
    let baud_rate = 115200;

    thread::spawn(move || {
        if let Ok(port) = serialport::new(port_name, baud_rate).open() {
            let mut reader = BufReader::new(port);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    let parts: Vec<&str> = line.trim().split(',').collect();
                    if parts.len() == 3 {
                        let min = parts[0].parse::<f32>().unwrap_or(0.0);
                        let max = parts[1].parse::<f32>().unwrap_or(1024.0);
                        let raw = parts[2].parse::<f32>().unwrap_or(0.0);
                        let _ = tx.send(PulseData { min, max, raw });
                    }
                }
            }
        }
    });

    let audio_host = audio::Host::new();
    let audio_model = Audio {
        phase: 0.0,
        hz: 220.0,
        target_hz: 220.0,
        amplitude: 0.0,
        target_amplitude: 0.0,
        lerp_factor: 0.1, // Controls how "snappy" the sound changes
    };
    
    let stream = audio_host
        .new_output_stream(audio_model)
        .render(audio_render)
        .build()
        .unwrap();

    stream.play().unwrap();

    Model {
        pulse: 0.0,
        receiver: rx,
        history: Vec::new(),
        stream,
        beat_detected: false,
    }
}

fn audio_render(audio: &mut Audio, buffer: &mut Buffer) {
    let sample_rate = buffer.sample_rate() as f64;
    
    for frame in buffer.frames_mut() {
        // Smooth transitions for frequency and amplitude to avoid clicking
        audio.hz += (audio.target_hz - audio.hz) * 0.005; 
        audio.amplitude += (audio.target_amplitude - audio.amplitude) * 0.01;

        // Generate a slightly more complex wave (Mix of Sine and Triangle)
        let sine = (2.0 * PI * audio.phase).sin();
        let triangle = (2.0 * (audio.phase % 1.0) - 1.0).abs() * 2.0 - 1.0;
        
        // Blend 80% sine, 20% triangle for a "warmer" medical tone
        let signal = (sine * 0.8 + triangle * 0.2) as f32;
        
        audio.phase += audio.hz / sample_rate;
        audio.phase %= 1.0;

        for channel in frame {
            *channel = signal * audio.amplitude * 0.9;
        }
    }
}
    
fn update(_app: &App, model: &mut Model, _update: Update) {
    while let Ok(data) = model.receiver.try_recv() {
        let range = data.max - data.min;
        let normalized = if range > 1.0 {
            ((data.raw - data.min) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        model.pulse = normalized;
        model.history.push(normalized);
        if model.history.len() > 500 {
            model.history.remove(0);
        }

        // Logic for "Beat Triggering"
        let p = model.pulse;
        model.stream.send(move |audio| {
            audio.target_hz = map_range(p as f64, 0.0, 1.0, 50.0, 150.0);
            
            // Adjusted exponent from 2.0 to 1.5 to make the sound curve less aggressive,
            // allowing the sound to be louder at mid-range pulse values.
            audio.target_amplitude = p.powf(1.5); 
        }).unwrap();
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();
    
    draw.background().color(BLACK);

    // Dynamic grid based on pulse
    let grid_size = 50.0;
    let alpha = 0.05 + (model.pulse * 0.1);
    for i in 0..20 {
        let x = win.left() + (i as f32 * grid_size * 2.0);
        draw.line().points(pt2(x, win.top()), pt2(x, win.bottom())).color(rgba(0.0, 1.0, 0.2, alpha));
    }

    // ECG Wave
    if model.history.len() > 1 {
        let points = (0..model.history.len()).map(|i| {
            let x = map_range(i, 0, 500, win.left(), win.right());
            let y = map_range(model.history[i], 0.0, 1.0, -100.0, 100.0);
            pt2(x, y)
        });

        draw.polyline()
            .weight(2.0 + (model.pulse * 5.0))
            .points(points)
            .color(GREENYELLOW);
    }

    // Heart indicator
    draw.ellipse()
        .xy(pt2(0.0, 200.0))
        .radius(50.0 + (model.pulse * 50.0))
        .color(rgba(1.0, 0.0, 0.1, 0.2 + model.pulse * 0.8))
        .stroke(WHITE)
        .stroke_weight(1.0);

    draw.to_frame(app, &frame).unwrap();
}