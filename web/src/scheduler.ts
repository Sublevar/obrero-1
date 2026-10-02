// Scheduler "A Tale of Two Clocks": un pump cada 25 ms pide al motor todo lo
// vencido en los próximos 100 ms y lo despacha con el send timestampeado de
// Web MIDI, que absorbe el jitter de setInterval (y el throttling en
// background hasta ~1 s).

import type { WasmEngine } from "./pkg/obrero_wasm";

const PUMP_INTERVAL_MS = 25;
const LOOKAHEAD_MS = 100;

// Registros planos [at_ms, status, d1, d2, len] * N que devuelve el motor.
export function sendAll(records: Float64Array, output: MIDIOutput | null): void {
  if (!output) return;
  for (let i = 0; i < records.length; i += 5) {
    const atMs = records[i];
    const len = records[i + 4];
    const bytes = Array.from(records.subarray(i + 1, i + 1 + len));
    output.send(bytes, atMs);
  }
}

export function startScheduler(
  engine: WasmEngine,
  getOutput: () => MIDIOutput | null,
  onPump?: (gapMs: number) => void,
): number {
  let last = performance.now();
  return window.setInterval(() => {
    const now = performance.now();
    const gap = now - last;
    last = now;
    const records = engine.advance(now, LOOKAHEAD_MS);
    sendAll(records, getOutput());
    onPump?.(gap);
  }, PUMP_INTERVAL_MS);
}
