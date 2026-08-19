// Acceso a dispositivos Web MIDI: enumeración, selección y hot-plug.

export interface MidiDevices {
  access: MIDIAccess;
  output: MIDIOutput | null;
  input: MIDIInput | null;
}

export async function initMidi(): Promise<MidiDevices> {
  const access = await navigator.requestMIDIAccess({ sysex: false });
  return { access, output: null, input: null };
}

export function listOutputs(access: MIDIAccess): MIDIOutput[] {
  return [...access.outputs.values()];
}

export function listInputs(access: MIDIAccess): MIDIInput[] {
  return [...access.inputs.values()];
}

export function selectOutput(devices: MidiDevices, id: string | null): void {
  devices.output = id ? (devices.access.outputs.get(id) ?? null) : null;
}

export function selectInput(
  devices: MidiDevices,
  id: string | null,
  onMessage: (data: Uint8Array, timeStampMs: number) => void,
): void {
  if (devices.input) {
    devices.input.onmidimessage = null;
  }
  devices.input = id ? (devices.access.inputs.get(id) ?? null) : null;
  if (devices.input) {
    devices.input.onmidimessage = (e: MIDIMessageEvent) => {
      if (e.data) onMessage(e.data, e.timeStamp);
    };
  }
}
