use obrero_core::clock::{DebugSink, DebugTap, TimeMark};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

const US_PER_MS: f64 = 1_000.0;

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

/// Consumidor de depuración del `Clock` del motor: loguea por `console.log`.
#[wasm_bindgen]
pub struct WasmDebugTap {
    tap: DebugTap,
}

impl WasmDebugTap {
    pub(crate) fn new(tap: DebugTap) -> Self {
        Self { tap }
    }
}

#[wasm_bindgen]
impl WasmDebugTap {
    /// Loguea los avisos pendientes; devuelve los perdidos por ring lleno.
    pub fn poll(&mut self) -> u32 {
        self.tap.poll(&mut ConsoleSink)
    }
}
