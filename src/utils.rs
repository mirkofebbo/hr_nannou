use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use crate::types::PulseData;

pub fn send_fallback_wave(tx: mpsc::Sender<PulseData>) {
    eprintln!("Arduino not detected - using a synthetic sine wave instead.");

    let mut phase = 0.0_f32;
    loop {
        let wobble = (phase * 0.37).sin() * 0.12;
        let normalized = (0.5 + 0.3 * phase.sin() + wobble).clamp(0.0, 1.0);
        let raw = normalized * 1024.0;

        if tx
            .send(PulseData {
                min: 0.0,
                max: 1024.0,
                raw,
            })
            .is_err()
        {
            break;
        }

        phase += 0.08;
        thread::sleep(Duration::from_millis(16));
    }
}