use obrero_core::engine::{Engine, TimedMidi};
use obrero_core::pattern::Step;
use obrero_core::{ChannelMode, ClockSource};
use wasm_bindgen::prelude::*;

const MS_TO_US: f64 = 1_000.0;

/// Wrapper wasm-bindgen del motor. Los timestamps cruzan la frontera en ms
/// `f64` (la base de tiempo de `performance.now()` / Web MIDI) y se convierten
/// a µs internamente.
#[wasm_bindgen]
pub struct WasmEngine {
    inner: Engine,
    buf: Vec<TimedMidi>,
}

/// Aplana los eventos como registros [at_ms, status, d1, d2, len] * N —
/// un solo Float64Array hacia JS, sin JSON en el camino caliente.
fn flatten(buf: &mut Vec<TimedMidi>) -> Vec<f64> {
    let mut flat = Vec::with_capacity(buf.len() * 5);
    for ev in buf.drain(..) {
        flat.push(ev.at_us as f64 / MS_TO_US);
        flat.push(ev.bytes[0] as f64);
        flat.push(ev.bytes[1] as f64);
        flat.push(ev.bytes[2] as f64);
        flat.push(ev.len as f64);
    }
    flat
}

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmEngine {
        WasmEngine {
            inner: Engine::new(),
            buf: Vec::new(),
        }
    }

    pub fn set_tempo(&mut self, bpm: f32) {
        self.inner.set_tempo(bpm);
    }

    pub fn play(&mut self) {
        self.inner.play();
    }

    pub fn stop(&mut self) {
        self.inner.stop();
    }

    pub fn set_clock_source(&mut self, external: bool) {
        self.inner.set_clock_source(if external {
            ClockSource::External
        } else {
            ClockSource::Internal
        });
    }

    pub fn toggle_step(&mut self, track: usize, step: usize) {
        self.inner.toggle_step(track, step);
    }

    pub fn set_step(
        &mut self,
        track: usize,
        step: usize,
        on: bool,
        note: u8,
        velocity: u8,
        gate_ticks: u8,
    ) {
        self.inner.set_step(
            track,
            step,
            Step {
                on,
                note,
                velocity,
                gate_ticks,
            },
        );
    }

    /// Canal MIDI 0-15 del track.
    pub fn set_track_channel(&mut self, track: usize, channel: u8) {
        self.inner.set_track_channel(track, channel);
    }

    /// Nota base del track (cambia la nota de todos sus pasos).
    pub fn set_track_note(&mut self, track: usize, note: u8) {
        self.inner.set_track_note(track, note);
    }

    /// true = todos los tracks salen por `channel` (modo drum machine);
    /// false = cada track usa su propio canal.
    pub fn set_channel_mode(&mut self, single: bool, channel: u8) {
        self.inner.set_channel_mode(if single {
            ChannelMode::Single(channel)
        } else {
            ChannelMode::PerTrack
        });
    }

    /// Emite todo lo vencido en (now, now+lookahead]. Devuelve registros
    /// planos [at_ms, status, d1, d2, len] * N.
    pub fn advance(&mut self, now_ms: f64, lookahead_ms: f64) -> Vec<f64> {
        self.inner.advance(
            (now_ms * MS_TO_US) as u64,
            (lookahead_ms * MS_TO_US) as u64,
            &mut self.buf,
        );
        flatten(&mut self.buf)
    }

    /// Entrada MIDI cruda (onmidimessage). Mismo formato de salida que advance.
    pub fn feed_midi_in(&mut self, bytes: &[u8], now_ms: f64) -> Vec<f64> {
        self.inner
            .feed_midi_in(bytes, (now_ms * MS_TO_US) as u64, &mut self.buf);
        flatten(&mut self.buf)
    }

    pub fn view(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.inner.view()).unwrap_or(JsValue::NULL)
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}
