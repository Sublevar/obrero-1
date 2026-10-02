use obrero_core::clock::TimeSource;
use wasm_bindgen::prelude::*;

// Bindings a mano en vez de `web-sys` (0.x, CLAUDE.md §1 "Dependencias").
// `performance` existe tanto en window como en workers.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub(crate) fn console_log(s: &str);
}

/// Reloj real de la web: `performance.now()` (ms `f64`) a µs.
pub struct WebTime;

impl TimeSource for WebTime {
    fn now_us(&self) -> u64 {
        (performance_now() * 1_000.0) as u64
    }
}
