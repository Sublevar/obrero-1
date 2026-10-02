use alloc::vec;
use alloc::vec::Vec;

pub use crate::tuning::PPQN;

/// Un paso de semicorchea.
pub const DEFAULT_TICKS_PER_STEP: u16 = (PPQN / 4) as u16;
/// Media semicorchea.
pub const DEFAULT_GATE_TICKS: u16 = (PPQN / 8) as u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub on: bool,
    pub note: u8,
    pub velocity: u8,
    /// Duración de la nota en ticks (`DEFAULT_TICKS_PER_STEP` = un paso).
    pub gate_ticks: u16,
}

/// Cómo se define la secuencia de un track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepMode {
    /// Pasos editados a mano en la grilla.
    Manual,
    /// Ritmo euclidiano: `pulses` golpes repartidos parejo en `steps` pasos.
    Euclidean { pulses: u8, steps: u8 },
}

/// E(k, n) por aritmética modular: el paso i golpea si (i·k) mod n < k.
/// Equivale a Bjorklund: E(3,8) = x..x..x. (tresillo), E(5,8) = x.x.xx.x.
pub fn euclidean_hits(pulses: u8, steps: u8) -> impl Iterator<Item = bool> {
    let n = steps.max(1) as u32;
    (0..n).map(move |i| euclidean_hit(i, pulses as u32, n))
}

/// Paso `i` de E(pulses, steps), consultable de a uno.
pub const fn euclidean_hit(i: u32, pulses: u32, steps: u32) -> bool {
    let n = if steps == 0 { 1 } else { steps };
    (i * pulses) % n < pulses
}

#[derive(Clone, Debug)]
pub struct Track {
    /// Canal MIDI 0-15.
    pub channel: u8,
    pub steps: Vec<Step>,
    /// Largo activo del track en pasos (permite polimetría; <= steps.len()).
    pub len: usize,
    pub muted: bool,
    pub mode: StepMode,
}

impl Track {
    pub fn new(channel: u8, note: u8, num_steps: usize) -> Self {
        Self {
            channel,
            steps: vec![
                Step {
                    on: false,
                    note,
                    velocity: 100,
                    gate_ticks: DEFAULT_GATE_TICKS
                };
                num_steps
            ],
            len: num_steps,
            muted: false,
            mode: StepMode::Manual,
        }
    }

    /// Regenera la grilla con E(pulses, steps) y acorta el track a `steps`.
    /// Conserva nota/velocidad/gate del track como base de los pasos nuevos.
    pub fn apply_euclidean(&mut self, pulses: u8, steps: u8) {
        let steps = steps.clamp(1, 64);
        let pulses = pulses.min(steps);
        let base = Step {
            on: false,
            ..self.steps.first().copied().unwrap_or(Step {
                on: false,
                note: 60,
                velocity: 100,
                gate_ticks: DEFAULT_GATE_TICKS,
            })
        };
        if self.steps.len() < steps as usize {
            self.steps.resize(steps as usize, base);
        }
        self.len = steps as usize;
        for (i, hit) in euclidean_hits(pulses, steps).enumerate() {
            self.steps[i].on = hit;
        }
        self.mode = StepMode::Euclidean { pulses, steps };
    }

    /// Vuelve a edición manual conservando la grilla generada y reabriendo
    /// el largo completo del track.
    pub fn set_manual(&mut self) {
        self.mode = StepMode::Manual;
        self.len = self.steps.len();
    }
}

/// Cómo se asigna el canal MIDI de cada track al emitir notas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelMode {
    /// Cada track sale por su propio canal (varios dispositivos).
    PerTrack,
    /// Todos los tracks salen por un canal único y se distinguen por nota
    /// (estilo drum machine: canal 10, una nota por instrumento).
    Single(u8),
}

/// Nota que el motor debe disparar en un tick dado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteEvent {
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
    pub gate_ticks: u16,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub tracks: Vec<Track>,
    /// Ticks por paso (`DEFAULT_TICKS_PER_STEP` = semicorcheas).
    pub ticks_per_step: u16,
    pub channel_mode: ChannelMode,
}

impl Pattern {
    /// Patrón inicial: 5 tracks de 16 pasos en canal 1, notas 60-64 (C4..E4).
    pub fn starter() -> Self {
        Self {
            tracks: (0..5).map(|i| Track::new(0, 60 + i, 16)).collect(),
            ticks_per_step: DEFAULT_TICKS_PER_STEP,
            channel_mode: ChannelMode::PerTrack,
        }
    }

    /// Notas que caen en `tick`. Única consulta que hace el motor durante la
    /// reproducción — reemplazar la grilla por una lista de eventos a futuro
    /// solo toca esta función.
    pub fn events_at(&self, tick: u64, out: &mut Vec<NoteEvent>) {
        let tps = self.ticks_per_step as u64;
        if !tick.is_multiple_of(tps) {
            return;
        }
        let step_index = (tick / tps) as usize;
        for track in &self.tracks {
            if track.muted || track.len == 0 {
                continue;
            }
            let step = track.steps[step_index % track.len];
            if step.on {
                let channel = match self.channel_mode {
                    ChannelMode::PerTrack => track.channel,
                    ChannelMode::Single(ch) => ch,
                };
                out.push(NoteEvent {
                    channel,
                    note: step.note,
                    velocity: step.velocity,
                    gate_ticks: step.gate_ticks,
                });
            }
        }
    }
}
