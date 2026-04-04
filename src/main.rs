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
    pulse: f32, // Normalized 0.0 to 1.0
    receiver: mpsc::Receiver<PulseData>,
    history: Vec<f32>,
}

fn model(app: &App) -> Model {
    app.new_window().size(800, 800).view(view).build().unwrap();

    // Setup Serial Communication in a separate thread
    let (tx, rx) = mpsc::channel();
    let port_name = "/dev/cu.usbmodemDC5475C4CF702"; // Your specific port
    let baud_rate = 115200;

    thread::spawn(move || {
        if let Ok(port) = serialport::new(port_name, baud_rate).open() {
            let mut reader = BufReader::new(port);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    // Arduino sends: displayMin,displayMax,rawValue
                    let parts: Vec<&str> = line.trim().split(',').collect();
                    if parts.len() == 3 {
                        let min = parts[0].parse::<f32>().unwrap_or(0.0);
                        let max = parts[1].parse::<f32>().unwrap_or(1023.0);
                        let raw = parts[2].parse::<f32>().unwrap_or(0.0);
                        
                        let _ = tx.send(PulseData { min, max, raw });
                    }
                }
            }
        } else {
            eprintln!("Failed to open port: {}", port_name);
        }
    });

    Model {
        pulse: 0.0,
        receiver: rx,
        history: Vec::new(),
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    // Check for new data from the Serial thread
    while let Ok(data) = model.receiver.try_recv() {
        // Calculate normalized pulse based on the dynamic range sent by Arduino
        let range = data.max - data.min;
        let normalized = if range > 0.0 {
            ((data.raw - data.min) / range).clamp(0.0, 1.0)
        } else {
            0.5
        };

        model.pulse = normalized;
        model.history.push(normalized);
        
        // Keep a longer history for a nicer wave (300 samples)
        if model.history.len() > 300 {
            model.history.remove(0);
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    
    // Create a dark "medical" theme
    draw.background().color(rgb(0.02, 0.02, 0.05));

    let win = app.window_rect();

    // 1. Draw a pulsing heart-like glow
    let radius = 150.0 + (model.pulse * 100.0);
    let color = rgba(0.9, 0.1, 0.2, 0.1 + (model.pulse * 0.4));
    
    // Outer glow
    draw.ellipse()
        .color(color)
        .w_h(radius * 1.5, radius * 1.5);
        
    // Core circle
    draw.ellipse()
        .color(rgb(0.8, 0.1, 0.1))
        .w_h(radius, radius)
        .stroke(WHITE)
        .stroke_weight(2.0);

    // 2. Draw the history wave (ECG Style)
    if model.history.len() > 1 {
        let points = (0..model.history.len()).map(|i| {
            let x = map_range(i, 0, 300, win.left() + 50.0, win.right() - 50.0);
            let y = map_range(model.history[i], 0.0, 1.0, win.bottom() + 150.0, win.bottom() + 350.0);
            pt2(x, y)
        });

        draw.polyline()
            .weight(3.0)
            .points(points)
            .color(GREENYELLOW);
    }

    draw.to_frame(app, &frame).unwrap();
}