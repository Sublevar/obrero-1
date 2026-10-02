use std::rc::Rc;

use obrero_core::clock::{
    Bpm, Clock, ClockError, DebugSink, DebugTap, Notice, Subdivision, Subscription, Time, TimeMark,
    TimeSource,
};
use wasm_bindgen::prelude::*;

use crate::time::{console_log, WebTime};

const US_PER_MS: f64 = 1_000.0;

fn js_err(e: ClockError) -> JsError {
    JsError::new(match e {
        ClockError::Full => "clock lleno: no quedan suscripciones libres",
        ClockError::OutOfRange => "subdivisión fuera de rango",
    })
}

/// `Clock` del core con su `Time` alimentado por `WebTime`.
#[wasm_bindgen]
pub struct WasmClock {
    clock: Rc<Clock>,
    time: Time,
    source: WebTime,
}

#[wasm_bindgen]
impl WasmClock {
    #[wasm_bindgen(constructor)]
    pub fn new(bpm: f32) -> WasmClock {
        WasmClock {
            clock: Clock::new(Bpm::from_f32(bpm)),
            time: Time::new(),
            source: WebTime,
        }
    }

    pub fn set_bpm(&self, bpm: f32) {
        self.clock.set_bpm(Bpm::from_f32(bpm));
    }

    pub fn bpm(&self) -> f32 {
        self.clock.bpm().as_f32()
    }

    pub fn set_precision_mode(&self, on: bool) {
        self.clock.set_precision_mode(on);
    }

    pub fn reset(&self) {
        self.clock.reset();
    }

    /// Lee el reloj real y genera los avisos hasta now + lookahead.
    pub fn advance(&mut self, lookahead_ms: f64) {
        self.time.advance(self.source.now_us());
        self.clock
            .advance(&self.time, (lookahead_ms * US_PER_MS) as u64);
    }

    pub fn skipped_ticks(&self) -> f64 {
        self.clock.skipped_ticks() as f64
    }

    pub fn subscribe_per_bar(&self, n: u8) -> Result<WasmSubscription, JsError> {
        self.subscribe(Subdivision::PerBar(n))
    }

    pub fn subscribe_every_n_bars(&self, n: u8) -> Result<WasmSubscription, JsError> {
        self.subscribe(Subdivision::EveryNBars(n))
    }

    /// Consumidor de depuración que loguea bar/beat por `console.log`.
    pub fn debug_tap(&self, per_bar: u8) -> Result<WasmDebugTap, JsError> {
        DebugTap::new(&self.clock, Subdivision::PerBar(per_bar))
            .map(|tap| WasmDebugTap { tap })
            .map_err(js_err)
    }
}

impl WasmClock {
    fn subscribe(&self, s: Subdivision) -> Result<WasmSubscription, JsError> {
        self.clock
            .subscribe(s)
            .map(|inner| WasmSubscription {
                inner,
                buf: Vec::new(),
                lost: 0,
            })
            .map_err(js_err)
    }
}

/// Suscripción expuesta a JS. Su `.free()` desuscribe (Drop del core).
#[wasm_bindgen]
pub struct WasmSubscription {
    inner: Subscription,
    buf: Vec<Notice>,
    lost: u32,
}

#[wasm_bindgen]
impl WasmSubscription {
    /// Registros planos [tick, at_ms] * N, mismo criterio que `advance` del
    /// motor: un Float64Array, sin JSON.
    pub fn take(&mut self) -> Vec<f64> {
        self.lost = self.inner.take(&mut self.buf);
        let mut flat = Vec::with_capacity(self.buf.len() * 2);
        for n in self.buf.drain(..) {
            flat.push(n.tick as f64);
            flat.push(n.at_us as f64 / US_PER_MS);
        }
        flat
    }

    /// Avisos perdidos por ring lleno en el último `take`.
    pub fn lost(&self) -> u32 {
        self.lost
    }

    /// Devuelve el n efectivo (puede diferir en modo precisión).
    pub fn set_per_bar(&self, n: u8) -> Result<u8, JsError> {
        match self.inner.set_subdivision(Subdivision::PerBar(n)) {
            Ok(Subdivision::PerBar(eff) | Subdivision::EveryNBars(eff)) => Ok(eff),
            Err(e) => Err(js_err(e)),
        }
    }

    pub fn set_every_n_bars(&self, n: u8) -> Result<u8, JsError> {
        match self.inner.set_subdivision(Subdivision::EveryNBars(n)) {
            Ok(Subdivision::PerBar(eff) | Subdivision::EveryNBars(eff)) => Ok(eff),
            Err(e) => Err(js_err(e)),
        }
    }
}

struct ConsoleSink;

impl DebugSink for ConsoleSink {
    fn emit(&mut self, m: TimeMark) {
        console_log(&format!(
            "[clock] bar {} beat {} tick {} @ {:.2} ms",
            m.bar,
            m.beat,
            m.tick_in_beat,
            m.at_us as f64 / US_PER_MS
        ));
    }
}

#[wasm_bindgen]
pub struct WasmDebugTap {
    tap: DebugTap,
}

#[wasm_bindgen]
impl WasmDebugTap {
    /// Loguea los avisos pendientes; devuelve los perdidos por ring lleno.
    pub fn poll(&mut self) -> u32 {
        self.tap.poll(&mut ConsoleSink)
    }
}
