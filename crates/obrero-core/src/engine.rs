use crate::input::{Button, InputEvent};
use crate::midi::MidiParser;
use crate::pattern::{ChannelMode, NoteEvent, Pattern, StepMode, PPQN};
use crate::view::{EuclideanView, StepView, TrackView, ViewModel};

pub const MIDI_CLOCK: u8 = 0xF8;
pub const MIDI_START: u8 = 0xFA;
pub const MIDI_CONTINUE: u8 = 0xFB;
pub const MIDI_STOP: u8 = 0xFC;

pub const MIN_BPM: f32 = 20.0;
pub const MAX_BPM: f32 = 300.0;

/// Mensaje MIDI con timestamp absoluto (µs monotónicos del caller).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimedMidi {
    pub at_us: u64,
    pub bytes: [u8; 3],
    pub len: u8,
}

impl TimedMidi {
    fn realtime(at_us: u64, status: u8) -> Self {
        Self {
            at_us,
            bytes: [status, 0, 0],
            len: 1,
        }
    }

    fn note_on(at_us: u64, channel: u8, note: u8, velocity: u8) -> Self {
        Self {
            at_us,
            bytes: [0x90 | (channel & 0x0F), note, velocity],
            len: 3,
        }
    }

    fn note_off(at_us: u64, channel: u8, note: u8) -> Self {
        Self {
            at_us,
            bytes: [0x80 | (channel & 0x0F), note, 0],
            len: 3,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockSource {
    Internal,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Transport {
    Stopped,
    Playing,
    /// Primer toque de Stop: deja terminar el paso en curso (su gate) y
    /// no dispara el próximo; el corte final llega cuando no queda nada
    /// sonando.
    Stopping,
}

#[derive(Clone, Copy, Debug)]
struct PendingOff {
    due_tick: u32,
    channel: u8,
    note: u8,
}

/// Motor sans-io: no posee threads, timers ni I/O. La plataforma lo hace
/// avanzar con tiempo explícito y recibe los eventos MIDI a emitir.
pub struct Engine {
    pattern: Pattern,
    clock_source: ClockSource,
    transport: Transport,
    bpm: f32,
    /// Próximo tick a disparar (contador desde Start).
    tick: u32,
    /// Timestamp del próximo tick en µs (f64 para acumular sin drift).
    /// None = sin anclar; se ancla al `now` del próximo `advance`.
    next_tick_at: Option<f64>,
    pending_offs: Vec<PendingOff>,
    start_pending: bool,
    stop_pending: bool,
    parser: MidiParser,
    selected_track: usize,
    note_buf: Vec<NoteEvent>,
}

impl Engine {
    pub fn new() -> Self {
        Self::with_pattern(Pattern::starter())
    }

    pub fn with_pattern(pattern: Pattern) -> Self {
        Self {
            pattern,
            clock_source: ClockSource::Internal,
            transport: Transport::Stopped,
            bpm: 120.0,
            tick: 0,
            next_tick_at: None,
            pending_offs: Vec::new(),
            start_pending: false,
            stop_pending: false,
            parser: MidiParser::new(),
            selected_track: 0,
            note_buf: Vec::new(),
        }
    }

    fn tick_period_us(&self) -> f64 {
        60_000_000.0 / (self.bpm as f64 * PPQN as f64)
    }

    // ── Superficie de control (UI de cualquier plataforma) ──────────────────

    pub fn set_tempo(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(MIN_BPM, MAX_BPM);
    }

    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    pub fn set_clock_source(&mut self, src: ClockSource) {
        if src == self.clock_source {
            return;
        }
        // Cambiar de fuente no permite seguir tickeando en la fuente vieja
        // para terminar el paso prolijamente: corte inmediato.
        self.force_stop();
        self.clock_source = src;
    }

    pub fn play(&mut self) {
        self.transport = Transport::Playing;
        self.tick = 0;
        self.pending_offs.clear();
        self.next_tick_at = None;
        self.start_pending = true;
        self.stop_pending = false;
    }

    /// Primer toque: si hay una nota sonando, la deja terminar su gate y
    /// no dispara el próximo paso (corte prolijo). Segundo toque mientras
    /// se está apagando: silencio general inmediato.
    pub fn stop(&mut self) {
        match self.transport {
            Transport::Stopped => {}
            Transport::Stopping => self.force_stop(),
            Transport::Playing => {
                self.start_pending = false;
                if self.pending_offs.is_empty() {
                    self.force_stop();
                } else {
                    self.transport = Transport::Stopping;
                }
            }
        }
    }

    /// Corte inmediato: apaga todo lo que esté sonando ya mismo y envía el
    /// Stop de transporte (si somos master). Usado por el doble toque de
    /// Stop y por cualquier cambio que no pueda esperar (p. ej. cambiar la
    /// fuente de reloj).
    fn force_stop(&mut self) {
        if self.transport != Transport::Stopped || !self.pending_offs.is_empty() {
            self.stop_pending = true;
        }
        self.transport = Transport::Stopped;
        self.start_pending = false;
        self.next_tick_at = None;
    }

    pub fn toggle_step(&mut self, track: usize, step: usize) {
        if let Some(t) = self.pattern.tracks.get_mut(track) {
            // En modo euclidiano la grilla la define E(k,n), no la mano.
            if matches!(t.mode, StepMode::Euclidean { .. }) {
                return;
            }
            if let Some(s) = t.steps.get_mut(step) {
                s.on = !s.on;
            }
        }
    }

    /// Activa modo euclidiano en el track: E(pulses, steps) regenera la grilla.
    pub fn set_track_euclidean(&mut self, track: usize, pulses: u8, steps: u8) {
        if let Some(t) = self.pattern.tracks.get_mut(track) {
            t.apply_euclidean(pulses, steps);
        }
    }

    /// Vuelve el track a edición manual (conserva la grilla generada).
    pub fn set_track_manual(&mut self, track: usize) {
        if let Some(t) = self.pattern.tracks.get_mut(track) {
            t.set_manual();
        }
    }

    /// Mutea/desmutea el track. Las notas ya disparadas apagan normalmente
    /// (sus note-offs siguen agendados).
    pub fn toggle_track_mute(&mut self, track: usize) {
        if let Some(t) = self.pattern.tracks.get_mut(track) {
            t.muted = !t.muted;
        }
    }

    pub fn set_step(&mut self, track: usize, step: usize, s: crate::pattern::Step) {
        if let Some(t) = self.pattern.tracks.get_mut(track) {
            if let Some(slot) = t.steps.get_mut(step) {
                *slot = s;
            }
        }
    }

    /// Canal MIDI 0-15. Las notas ya sonando apagan en el canal viejo
    /// (los note-offs pendientes guardan su canal).
    pub fn set_track_channel(&mut self, track: usize, channel: u8) {
        if let Some(t) = self.pattern.tracks.get_mut(track) {
            t.channel = channel & 0x0F;
        }
    }

    /// Cambia la nota de todos los pasos del track (lane estilo drum machine).
    pub fn set_track_note(&mut self, track: usize, note: u8) {
        if let Some(t) = self.pattern.tracks.get_mut(track) {
            for s in &mut t.steps {
                s.note = note & 0x7F;
            }
        }
    }

    pub fn set_channel_mode(&mut self, mode: ChannelMode) {
        self.pattern.channel_mode = match mode {
            ChannelMode::Single(ch) => ChannelMode::Single(ch & 0x0F),
            ChannelMode::PerTrack => ChannelMode::PerTrack,
        };
    }

    pub fn pattern_mut(&mut self) -> &mut Pattern {
        &mut self.pattern
    }

    pub fn handle_input(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::ButtonDown(Button::Play) => self.play(),
            InputEvent::ButtonDown(Button::Stop) => self.stop(),
            InputEvent::ButtonDown(Button::Step(n)) => {
                self.toggle_step(self.selected_track, n as usize)
            }
            InputEvent::ButtonDown(Button::TrackSelect(n)) => {
                if (n as usize) < self.pattern.tracks.len() {
                    self.selected_track = n as usize;
                }
            }
            InputEvent::EncoderDelta(d) => self.set_tempo(self.bpm + d as f32),
            _ => {}
        }
    }

    // ── Corazón sans-io ──────────────────────────────────────────────────────

    /// Avanza el reloj interno y emite todo lo vencido en (now, now+lookahead].
    /// El estado se consume al emitir, así que ventanas solapadas nunca
    /// duplican eventos. ESP32 llama con lookahead 0; la web con ~100 ms.
    pub fn advance(&mut self, now_us: u64, lookahead_us: u64, out: &mut Vec<TimedMidi>) {
        if self.stop_pending {
            self.stop_pending = false;
            if self.clock_source == ClockSource::Internal {
                out.push(TimedMidi::realtime(now_us, MIDI_STOP));
            }
            self.flush_pending_offs(now_us, out);
        }

        if self.clock_source != ClockSource::Internal || self.transport == Transport::Stopped {
            return;
        }

        if self.start_pending {
            self.start_pending = false;
            out.push(TimedMidi::realtime(now_us, MIDI_START));
            self.next_tick_at = Some(now_us as f64);
        }

        let horizon = (now_us + lookahead_us) as f64;
        while let Some(t) = self.next_tick_at {
            if t > horizon {
                break;
            }
            self.on_tick(t as u64, true, out);
            self.next_tick_at = Some(t + self.tick_period_us());

            if self.transport == Transport::Stopping && self.pending_offs.is_empty() {
                // Ya no queda nada sonando: recién ahora cortamos.
                self.transport = Transport::Stopped;
                self.next_tick_at = None;
                out.push(TimedMidi::realtime(t as u64, MIDI_STOP));
                break;
            }
        }
    }

    /// Próximo instante en que hay trabajo pendiente (para que el firmware
    /// pueda dormir con precisión). None = nada agendado.
    pub fn next_event_at(&self) -> Option<u64> {
        if self.stop_pending || self.start_pending {
            return Some(0);
        }
        if self.clock_source == ClockSource::Internal && self.transport != Transport::Stopped {
            return self.next_tick_at.map(|t| t as u64);
        }
        None
    }

    /// Entrada MIDI cruda. Parsea bytes; en modo External, los mensajes de
    /// reloj/transporte (0xF8/0xFA/0xFB/0xFC) manejan el secuenciador y las
    /// notas resultantes salen estampadas con `now_us`.
    pub fn feed_midi_in(&mut self, bytes: &[u8], now_us: u64, out: &mut Vec<TimedMidi>) {
        for &b in bytes {
            let Some((status, _d1, _d2)) = self.parser.feed(b) else {
                continue;
            };
            if self.clock_source != ClockSource::External {
                continue;
            }
            match status {
                MIDI_START => {
                    self.tick = 0;
                    self.pending_offs.clear();
                    self.transport = Transport::Playing;
                }
                MIDI_CONTINUE => {
                    self.transport = Transport::Playing;
                }
                MIDI_STOP => {
                    self.transport = Transport::Stopped;
                    self.flush_pending_offs(now_us, out);
                }
                MIDI_CLOCK => {
                    if self.transport != Transport::Stopped {
                        // Como esclavo no re-emitimos 0xF8.
                        self.on_tick(now_us, false, out);
                        if self.transport == Transport::Stopping && self.pending_offs.is_empty() {
                            self.transport = Transport::Stopped;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn view(&self) -> ViewModel {
        let tps = self.pattern.ticks_per_step as u32;
        let last_tick = self.tick.saturating_sub(1);
        // Paso global desde Start; cada track lo reduce módulo su propio largo.
        let global_step = if self.transport != Transport::Stopped && self.tick > 0 {
            Some((last_tick / tps) as usize)
        } else {
            None
        };
        let step_in = |len: usize| match global_step {
            Some(g) if len > 0 => g % len,
            _ => 0,
        };
        let current_step = step_in(
            self.pattern
                .tracks
                .get(self.selected_track)
                .map(|t| t.len)
                .unwrap_or(0),
        );
        ViewModel {
            playing: self.transport != Transport::Stopped,
            bpm: self.bpm,
            external_clock: self.clock_source == ClockSource::External,
            single_channel: match self.pattern.channel_mode {
                ChannelMode::Single(ch) => Some(ch),
                ChannelMode::PerTrack => None,
            },
            current_step,
            selected_track: self.selected_track,
            tracks: self
                .pattern
                .tracks
                .iter()
                .map(|t| TrackView {
                    channel: t.channel,
                    muted: t.muted,
                    note: t.steps.first().map(|s| s.note).unwrap_or(0),
                    euclidean: match t.mode {
                        StepMode::Euclidean { pulses, steps } => {
                            Some(EuclideanView { pulses, steps })
                        }
                        StepMode::Manual => None,
                    },
                    current_step: step_in(t.len),
                    steps: t.steps[..t.len]
                        .iter()
                        .map(|s| StepView {
                            on: s.on,
                            note: s.note,
                            velocity: s.velocity,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    // ── Internos ─────────────────────────────────────────────────────────────

    /// Un tick de reloj (interno o externo): note-offs vencidos, note-ons del
    /// tick, y 0xF8 si actuamos de master.
    fn on_tick(&mut self, at_us: u64, send_clock: bool, out: &mut Vec<TimedMidi>) {
        if send_clock {
            out.push(TimedMidi::realtime(at_us, MIDI_CLOCK));
        }
        let tick = self.tick;

        self.pending_offs.retain(|p| {
            if p.due_tick <= tick {
                out.push(TimedMidi::note_off(at_us, p.channel, p.note));
                false
            } else {
                true
            }
        });

        // En modo Stopping no se dispara el próximo paso: solo se dejan
        // terminar las notas ya en curso.
        if self.transport == Transport::Playing {
            self.note_buf.clear();
            self.pattern.events_at(tick, &mut self.note_buf);
            for ev in &self.note_buf {
                out.push(TimedMidi::note_on(at_us, ev.channel, ev.note, ev.velocity));
                self.pending_offs.push(PendingOff {
                    due_tick: tick + ev.gate_ticks as u32,
                    channel: ev.channel,
                    note: ev.note,
                });
            }
        }

        self.tick = tick + 1;
    }

    fn flush_pending_offs(&mut self, at_us: u64, out: &mut Vec<TimedMidi>) {
        for p in self.pending_offs.drain(..) {
            out.push(TimedMidi::note_off(at_us, p.channel, p.note));
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
