use nannou::prelude::*;
use std::f32::MAX_10_EXP;
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

struct Model {
    pulse: f32,
    receiver: mpsc::Receiver<PulseData>,
    history: Vec<f32>,
}

fn model(app: &App) -> Model {
    app.new_window()
        .size(800, 800)
        .title("Heart Action - Visualizer")
        .view(view)
        .build()
        .unwrap();

    let (tx, rx) = mpsc::channel();

    // Adjust port for your specific Arduino setup
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

    Model {
        pulse: 0.0,
        receiver: rx,
        history: Vec::new(),
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

        // Maintain a history of 500 samples for the wave
        if model.history.len() > 500 {
            model.history.remove(0);
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();

    draw.background().color(BLACK);

    // 2. ECG Style Wave
    const NUM_RINGS: usize = 20;
    if model.history.len() > 1 {
        for r in 0..NUM_RINGS {
            let _r = r as f32;
            let pulse_offset = model.pulse * _r + 1.0; // Adjust pulse influence on radius
            let points = (0..model.history.len()).map(|i| {
                let min_radius = map_range(_r, 0.0, NUM_RINGS as f32, 0.0, win.w() / 2.0);
                let max_radius = map_range(_r, 0.0, NUM_RINGS as f32, win.w() / 2.0, win.w() / 1.5);
                let radius = map_range(model.history[i], 0.0, 1.0, min_radius, max_radius);
                // let a = (i as f32 * 360.0 / model.history.len() as f32 + _r / pulse_offset)
                let a = (i as f32 * 360.0 / model.history.len() as f32)
                    .to_radians();
                let x = radius * a.cos();
                let y = radius * a.sin();
                pt2(x, y)
            });

            draw.polyline()
                .weight(pulse_offset)
                .points(points)
                .rgba(
                    1.0,
                    1.0,
                    1.0,
                    map_range(r as f32, 0.0, NUM_RINGS as f32, 0.1, 1.0),
                );
        }
    }

    // // 3. Central Heart Pulse Indicator
    // let heart_size = 60.0 + (model.pulse * 60.0);
    // draw.ellipse()
    //     .xy(pt2(0.0, 0.0))
    //     .radius(heart_size)
    //     .no_fill()
    //     .stroke(WHITE)
    //     .stroke_weight(2.0 + (model.pulse * 4.0));

    draw.to_frame(app, &frame).unwrap();
}
