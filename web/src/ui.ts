// UI de grilla de pasos. El core no dibuja nada: acá se renderiza el
// ViewModel que expone el motor y se traducen los clicks a llamadas al engine.

export interface StepView {
  on: boolean;
  note: number;
  velocity: number;
}

export interface TrackView {
  channel: number;
  muted: boolean;
  note: number;
  /** Parámetros euclidianos; null/ausente = modo manual. */
  euclidean?: { pulses: number; steps: number } | null;
  /** Paso sonando de este track (los largos pueden diferir). */
  current_step: number;
  steps: StepView[];
}

export interface ViewModel {
  playing: boolean;
  bpm: number;
  external_clock: boolean;
  /** Canal global en modo único (drum machine); null = canal por track. */
  single_channel: number | null;
  current_step: number;
  selected_track: number;
  tracks: TrackView[];
}

export interface UiCallbacks {
  onPlay(): void;
  onStop(): void;
  onToggleStep(track: number, step: number): void;
  onTempo(bpm: number): void;
  onClockSource(external: boolean): void;
  onSelectOutput(id: string | null): void;
  onSelectInput(id: string | null): void;
  onTrackChannel(track: number, channel: number): void;
  onTrackNote(track: number, note: number): void;
  onChannelMode(single: boolean, channel: number): void;
  onTrackEuclidean(track: number, pulses: number, steps: number): void;
  onTrackManual(track: number): void;
  onTrackMute(track: number): void;
}

const TRACK_LABELS: string[] = [];

export class Ui {
  private grid: HTMLButtonElement[][] = [];
  private channelSelects: HTMLSelectElement[] = [];
  private noteInputs: HTMLInputElement[] = [];
  private modeSelects: HTMLSelectElement[] = [];
  private pulsesInputs: HTMLInputElement[] = [];
  private stepsInputs: HTMLInputElement[] = [];
  private muteButtons: HTMLButtonElement[] = [];
  private playBtn!: HTMLButtonElement;
  private bpmInput!: HTMLInputElement;
  private clockSelect!: HTMLSelectElement;
  private modeSelect!: HTMLSelectElement;
  private globalChannelSelect!: HTMLSelectElement;
  private globalChannelLabel!: HTMLElement;
  private outputSelect!: HTMLSelectElement;
  private inputSelect!: HTMLSelectElement;
  private statusEl!: HTMLElement;

  constructor(root: HTMLElement, numTracks: number, numSteps: number, cb: UiCallbacks) {
    root.innerHTML = `
      <h1>Guiso</h1>
      <p class="tagline">toda la magia midi en una misma olla</p>
      <div class="transport">
        <button id="play">▶ Play</button>
        <button id="stop">■ Stop</button>
        <label>BPM <input id="bpm" type="number" min="20" max="300" value="120" /></label>
        <label>Reloj
          <select id="clock">
            <option value="internal">Interno</option>
            <option value="external">Externo (MIDI IN)</option>
          </select>
        </label>
      </div>
      <div class="devices">
        <label>Salida MIDI <select id="midi-out"></select></label>
        <label>Entrada MIDI <select id="midi-in"></select></label>
        <label>Canales
          <select id="chmode">
            <option value="per-track">Por track</option>
            <option value="single">Único (drum machine)</option>
          </select>
        </label>
        <label id="global-ch-label" hidden>Canal global <select id="global-ch"></select></label>
      </div>
      <div id="grid" class="grid"></div>
      <p id="status" class="status"></p>
    `;

    this.playBtn = root.querySelector("#play")!;
    this.bpmInput = root.querySelector("#bpm")!;
    this.clockSelect = root.querySelector("#clock")!;
    this.modeSelect = root.querySelector("#chmode")!;
    this.globalChannelSelect = root.querySelector("#global-ch")!;
    this.globalChannelLabel = root.querySelector("#global-ch-label")!;
    this.outputSelect = root.querySelector("#midi-out")!;
    this.inputSelect = root.querySelector("#midi-in")!;
    this.statusEl = root.querySelector("#status")!;

    for (let ch = 0; ch < 16; ch++) {
      const opt = document.createElement("option");
      opt.value = String(ch);
      opt.textContent = `Ch ${ch + 1}`;
      this.globalChannelSelect.appendChild(opt);
    }
    this.globalChannelSelect.value = "0"; // canal 1

    const notifyChannelMode = () =>
      cb.onChannelMode(
        this.modeSelect.value === "single",
        Number(this.globalChannelSelect.value),
      );
    this.modeSelect.addEventListener("change", notifyChannelMode);
    this.globalChannelSelect.addEventListener("change", notifyChannelMode);

    this.playBtn.addEventListener("click", () => cb.onPlay());
    root.querySelector("#stop")!.addEventListener("click", () => cb.onStop());
    this.bpmInput.addEventListener("change", () => cb.onTempo(Number(this.bpmInput.value)));
    this.clockSelect.addEventListener("change", () =>
      cb.onClockSource(this.clockSelect.value === "external"),
    );
    this.outputSelect.addEventListener("change", () =>
      cb.onSelectOutput(this.outputSelect.value || null),
    );
    this.inputSelect.addEventListener("change", () =>
      cb.onSelectInput(this.inputSelect.value || null),
    );

    const grid = root.querySelector("#grid")!;
    for (let t = 0; t < numTracks; t++) {
      const row = document.createElement("div");
      row.className = "row";

      // Tres grupos por fila: cabecera, tiles y config — en pantallas
      // angostas los tiles fluyen a su propia línea y warpean.
      const head = document.createElement("div");
      head.className = "track-head";

      const label = document.createElement("span");
      label.className = "label";
      label.textContent = TRACK_LABELS[t] ?? `Track ${t + 1}`;
      head.appendChild(label);

      const mute = document.createElement("button");
      mute.className = "mute";
      mute.textContent = "M";
      mute.title = "Mutear track";
      mute.addEventListener("click", () => cb.onTrackMute(t));
      head.appendChild(mute);
      this.muteButtons.push(mute);

      // Canal MIDI del track: 1-16 en pantalla, 0-15 hacia el motor.
      const channel = document.createElement("select");
      channel.className = "channel";
      channel.title = "Canal MIDI";
      for (let ch = 0; ch < 16; ch++) {
        const opt = document.createElement("option");
        opt.value = String(ch);
        opt.textContent = `Ch ${ch + 1}`;
        channel.appendChild(opt);
      }
      channel.addEventListener("change", () => cb.onTrackChannel(t, Number(channel.value)));
      head.appendChild(channel);
      this.channelSelects.push(channel);

      const cellsBox = document.createElement("div");
      cellsBox.className = "cells";
      const cells: HTMLButtonElement[] = [];
      for (let s = 0; s < numSteps; s++) {
        const cell = document.createElement("button");
        cell.className = "cell" + (s % 4 === 0 ? " beat" : "");
        cell.addEventListener("click", () => cb.onToggleStep(t, s));
        cellsBox.appendChild(cell);
        cells.push(cell);
      }
      this.grid.push(cells);

      // Rueda de config al final de la fila: modo manual / euclidiano E(k,n)
      const gear = document.createElement("button");
      gear.className = "gear";
      gear.textContent = "⚙";
      gear.title = "Configurar track";

      const config = document.createElement("span");
      config.className = "config";
      config.hidden = true;
      gear.addEventListener("click", () => {
        config.hidden = !config.hidden;
      });

      // Nota MIDI del track (lane drum machine: una nota por instrumento).
      const noteLabel = document.createElement("label");
      noteLabel.textContent = "Nota ";
      const note = document.createElement("input");
      note.type = "number";
      note.className = "note";
      note.min = "0";
      note.max = "127";
      note.title = "Nota MIDI";
      note.addEventListener("change", () => cb.onTrackNote(t, Number(note.value)));
      noteLabel.appendChild(note);
      this.noteInputs.push(note);

      const mode = document.createElement("select");
      for (const [value, text] of [
        ["manual", "Manual"],
        ["euclid", "Euclidiano"],
      ]) {
        const opt = document.createElement("option");
        opt.value = value;
        opt.textContent = text;
        mode.appendChild(opt);
      }

      const pulses = document.createElement("input");
      pulses.type = "number";
      pulses.className = "euclid-param";
      pulses.min = "0";
      pulses.max = String(numSteps);
      pulses.value = "4";
      pulses.title = "Pulsos (k)";

      const steps = document.createElement("input");
      steps.type = "number";
      steps.className = "euclid-param";
      steps.min = "1";
      steps.max = String(numSteps);
      steps.value = String(numSteps);
      steps.title = "Pasos (n)";

      const applyMode = () => {
        if (mode.value === "euclid") {
          cb.onTrackEuclidean(t, Number(pulses.value), Number(steps.value));
        } else {
          cb.onTrackManual(t);
        }
      };
      mode.addEventListener("change", applyMode);
      pulses.addEventListener("change", applyMode);
      steps.addEventListener("change", applyMode);

      config.append(noteLabel, mode, pulses, steps);
      const tail = document.createElement("div");
      tail.className = "track-tail";
      tail.append(gear, config);
      row.append(head, cellsBox, tail);
      this.modeSelects.push(mode);
      this.pulsesInputs.push(pulses);
      this.stepsInputs.push(steps);

      grid.appendChild(row);
    }
  }

  setDevices(outputs: { id: string; name: string }[], inputs: { id: string; name: string }[]): void {
    fillSelect(this.outputSelect, outputs);
    fillSelect(this.inputSelect, inputs);
  }

  setStatus(msg: string): void {
    this.statusEl.textContent = msg;
  }

  render(vm: ViewModel): void {
    this.playBtn.classList.toggle("active", vm.playing);
    if (document.activeElement !== this.bpmInput) {
      this.bpmInput.value = String(vm.bpm);
    }
    const single = vm.single_channel !== null;
    this.globalChannelLabel.hidden = !single;
    if (document.activeElement !== this.modeSelect) {
      this.modeSelect.value = single ? "single" : "per-track";
    }
    if (single && document.activeElement !== this.globalChannelSelect) {
      this.globalChannelSelect.value = String(vm.single_channel);
    }
    for (let t = 0; t < this.grid.length; t++) {
      this.muteButtons[t].classList.toggle("active", vm.tracks[t]?.muted ?? false);
      const channelSelect = this.channelSelects[t];
      // En modo canal único los selects por track no aplican
      channelSelect.hidden = single;
      if (vm.tracks[t] && document.activeElement !== channelSelect) {
        channelSelect.value = String(vm.tracks[t].channel);
      }
      const noteInput = this.noteInputs[t];
      if (vm.tracks[t] && document.activeElement !== noteInput) {
        noteInput.value = String(vm.tracks[t].note);
      }

      const euclid = vm.tracks[t]?.euclidean ?? null;
      const modeSelect = this.modeSelects[t];
      if (document.activeElement !== modeSelect) {
        modeSelect.value = euclid ? "euclid" : "manual";
      }
      const pulsesInput = this.pulsesInputs[t];
      const stepsInput = this.stepsInputs[t];
      pulsesInput.hidden = !euclid;
      stepsInput.hidden = !euclid;
      if (euclid && document.activeElement !== pulsesInput) {
        pulsesInput.value = String(euclid.pulses);
      }
      if (euclid && document.activeElement !== stepsInput) {
        stepsInput.value = String(euclid.steps);
      }

      const steps = vm.tracks[t]?.steps ?? [];
      for (let s = 0; s < this.grid[t].length; s++) {
        const cell = this.grid[t][s];
        // La secuencia generada se ve en la grilla; los pasos fuera del
        // largo del track se ocultan y en modo euclidiano no se editan.
        cell.hidden = s >= steps.length;
        cell.classList.toggle("locked", !!euclid);
        cell.classList.toggle("on", steps[s]?.on ?? false);
        cell.classList.toggle(
          "playhead",
          vm.playing && s === (vm.tracks[t]?.current_step ?? 0),
        );
      }
    }
  }
}

function fillSelect(select: HTMLSelectElement, items: { id: string; name: string }[]): void {
  const previous = select.value;
  select.innerHTML = '<option value="">— ninguno —</option>';
  for (const item of items) {
    const opt = document.createElement("option");
    opt.value = item.id;
    opt.textContent = item.name;
    select.appendChild(opt);
  }
  if ([...select.options].some((o) => o.value === previous)) {
    select.value = previous;
  }
}
