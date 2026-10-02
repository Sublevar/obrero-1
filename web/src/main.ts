import init, { WasmEngine } from "./pkg/obrero_wasm";
import { initMidi, listInputs, listOutputs, selectInput, selectOutput } from "./midi";
import { LossMonitor } from "./lossmonitor";
import { sendAll, startScheduler } from "./scheduler";
import { Ui, type ViewModel } from "./ui";

const NUM_TRACKS = 5;
const NUM_STEPS = 16;

async function main(): Promise<void> {
  await init();
  const engine = new WasmEngine();
  const root = document.querySelector<HTMLElement>("#app")!;

  const params = new URLSearchParams(location.search);
  const monitor = new LossMonitor(params.get("losslog") === "off");
  (window as unknown as { obreroLoss: LossMonitor }).obreroLoss = monitor;
  if (params.has("clockdebug")) {
    startClockDebug(engine, monitor);
  }

  let devices: Awaited<ReturnType<typeof initMidi>> | null = null;

  const onMidiMessage = (data: Uint8Array, timeStampMs: number): void => {
    const records = engine.feed_midi_in(data, timeStampMs);
    sendAll(records, devices?.output ?? null);
  };

  const ui = new Ui(root, NUM_TRACKS, NUM_STEPS, {
    onPlay: () => engine.play(),
    onStop: () => engine.stop(),
    onToggleStep: (t, s) => engine.toggle_step(t, s),
    onTempo: (bpm) => engine.set_tempo(bpm),
    onClockSource: (external) => engine.set_clock_source(external),
    onSelectOutput: (id) => devices && selectOutput(devices, id),
    onSelectInput: (id) => devices && selectInput(devices, id, onMidiMessage),
    onTrackChannel: (t, ch) => engine.set_track_channel(t, ch),
    onTrackNote: (t, note) => engine.set_track_note(t, note),
    onChannelMode: (single, ch) => engine.set_channel_mode(single, ch),
    onTrackEuclidean: (t, k, n) => engine.set_track_euclidean(t, k, n),
    onTrackManual: (t) => engine.set_track_manual(t),
    onTrackMute: (t) => engine.toggle_track_mute(t),
  });

  try {
    devices = await initMidi();
  } catch (err) {
    ui.setStatus(`Web MIDI no disponible: ${err}. Usá Chrome/Edge sobre HTTPS o localhost.`);
    return;
  }

  const refreshDevices = (): void => {
    if (!devices) return;
    ui.setDevices(
      listOutputs(devices.access).map((o) => ({ id: o.id, name: o.name ?? o.id })),
      listInputs(devices.access).map((i) => ({ id: i.id, name: i.name ?? i.id })),
    );
  };
  devices.access.onstatechange = refreshDevices;
  refreshDevices();
  ui.setStatus("Elegí una salida MIDI y dale Play.");

  startScheduler(engine, () => devices?.output ?? null, (gap) => monitor.poll(engine, gap));

  const draw = (): void => {
    ui.render(engine.view() as ViewModel);
    requestAnimationFrame(draw);
  };
  requestAnimationFrame(draw);
}

// Modo debug (`?clockdebug`): cuelga un DebugTap del Clock propio del motor y
// muestra bpm, la marca de tiempo del próximo tick y un indicador de pérdidas.
// `next_tick_ms` es el próximo tick que el reloj va a generar (queda ~lookahead
// por delante de `now`); sus diferencias entre pumps deben ser un período de
// tick exacto. Las pérdidas (ticks saltados, pasos sin sonar, ticks descartados,
// avisos del tap perdidos) las avisa `LossMonitor` por consola y, en `vite dev`,
// al servidor; el botón del panel (o `window.obreroLoss.muted`) las duerme.
// Expuesto en `window.obreroEngine` para cambiar el tempo desde la consola.
function startClockDebug(engine: WasmEngine, monitor: LossMonitor): void {
  (window as unknown as { obreroEngine: WasmEngine }).obreroEngine = engine;
  const tap = engine.debug_tap(4);
  const panel = document.createElement("div");
  panel.style.cssText =
    "position:fixed;right:8px;bottom:8px;padding:6px 8px;font:12px monospace;" +
    "background:#000c;color:#0f0;z-index:9999";
  const text = document.createElement("pre");
  text.style.margin = "0";
  const mute = document.createElement("button");
  mute.style.cssText = "margin-top:4px;font:12px monospace;cursor:pointer";
  mute.addEventListener("click", () => {
    monitor.muted = !monitor.muted;
  });
  panel.append(text, mute);
  document.body.appendChild(panel);

  let lastNext = NaN;
  window.setInterval(() => {
    monitor.addLostNotices(tap.poll());
    const next = engine.next_tick_ms();
    if (next !== lastNext) {
      console.log(
        `[clock] bpm=${engine.bpm().toFixed(2)} próximo tick @ ${next.toFixed(3)} ms ` +
          `(now ${performance.now().toFixed(3)} ms, +${(next - performance.now()).toFixed(3)} ms)`,
      );
      lastNext = next;
    }
  }, 25);

  const paint = (): void => {
    const now = performance.now();
    const next = engine.next_tick_ms();
    const t = monitor.totals;
    text.textContent =
      `bpm            ${engine.bpm().toFixed(2)}\n` +
      `now            ${now.toFixed(3)} ms\n` +
      `próximo tick @ ${Number.isNaN(next) ? "—" : next.toFixed(3) + " ms"}\n` +
      `Δ              ${Number.isNaN(next) ? "—" : (next - now).toFixed(3) + " ms"}\n` +
      `pump           ${t.lastGapMs.toFixed(1)} ms (máx ${t.maxGapMs.toFixed(1)})\n` +
      `pérdidas       ${monitor.lossy ? "SÍ" : "ninguna"}\n` +
      `  ticks saltados     ${t.skippedTicks}\n` +
      `  pasos sin sonar    ${t.skippedSteps}\n` +
      `  ticks descartados  ${t.lostTicks}\n` +
      `  avisos del tap     ${t.lostNotices}`;
    text.style.color = monitor.lossy ? "#f55" : "#0f0";
    mute.textContent = monitor.muted ? "avisos dormidos (activar)" : "dormir avisos";
    requestAnimationFrame(paint);
  };
  requestAnimationFrame(paint);
}

main();
