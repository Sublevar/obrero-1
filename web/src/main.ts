import init, { WasmEngine } from "./pkg/obrero_wasm";
import { initMidi, listInputs, listOutputs, selectInput, selectOutput } from "./midi";
import { sendAll, startScheduler } from "./scheduler";
import { Ui, type ViewModel } from "./ui";

const NUM_TRACKS = 5;
const NUM_STEPS = 16;

async function main(): Promise<void> {
  await init();
  const engine = new WasmEngine();
  const root = document.querySelector<HTMLElement>("#app")!;

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

  startScheduler(engine, () => devices?.output ?? null);

  const draw = (): void => {
    ui.render(engine.view() as ViewModel);
    requestAnimationFrame(draw);
  };
  requestAnimationFrame(draw);
}

main();
