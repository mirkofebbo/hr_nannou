use nannou::prelude::*;
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
    const SMOOTHING: f32 = 0.02;

    while let Ok(data) = model.receiver.try_recv() {
        let range = data.max - data.min;
        let normalized = if range > 1.0 {
            ((data.raw - data.min) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        model.pulse += (normalized - model.pulse) * SMOOTHING;
        model.history.push(model.pulse);

        if model.history.len() > 500 {
            model.history.remove(0);
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();

    draw.background().color(BLACK);

    const NUM_TRI: usize = 20;
    if !model.history.is_empty() {
        let history_len = model.history.len();

        for j in 0..NUM_TRI {
            let pulse_offset = model.pulse * j as f32 + 1.0;
            let triangle_offset = j * (history_len/10) * history_len / NUM_TRI;

            let points = (0..=3).map(|vertex| {
                let min_radius = map_range(j as f32, 0.0, NUM_TRI as f32, 0.0, win.w() / 2.0);
                let max_radius = map_range(j as f32, 0.0, NUM_TRI as f32, win.w() / 2.0, win.w() / 1.5);

                let vertex_offset = (vertex % 3) * history_len / 3;
                let sample_idx = (triangle_offset + vertex_offset) % history_len;
                let smooth_window = 6.min(history_len);
                let smoothed_sample = (0..smooth_window)
                    .map(|k| model.history[(sample_idx + k) % history_len])
                    .sum::<f32>()
                    / smooth_window as f32;
                let radius = map_range(smoothed_sample, 0.0, 1.0, min_radius, max_radius);

                let mut angle = (vertex as f32 * 360.0 / 3.0).to_radians();
                angle += pulse_offset * 0.05;
                let x = radius * angle.cos();
                let y = radius * angle.sin();
                pt2(x, y)
            });

            draw.polyline()
                .weight(0.75 + pulse_offset * 0.15)
                .points(points)
                .rgba(
                    1.0,
                    1.0,
                    1.0,
                    map_range(j as f32, 0.0, NUM_TRI as f32, 0.1, 1.0),
                );
        }
    }

    draw.to_frame(app, &frame).unwrap();
}
