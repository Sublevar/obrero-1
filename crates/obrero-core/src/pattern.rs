/// Resolución interna: 24 ticks por negra (= MIDI clock).
pub const PPQN: u32 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub on: bool,
    pub note: u8,
    pub velocity: u8,
    /// Duración de la nota en ticks (6 ticks = un paso de semicorchea).
    pub gate_ticks: u8,
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
    let k = pulses as u32;
    let n = steps.max(1) as u32;
    (0..n).map(move |i| (i * k) % n < k)
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
                    gate_ticks: 3
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
                gate_ticks: 3,
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
    pub gate_ticks: u8,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub tracks: Vec<Track>,
    /// Ticks por paso (6 = semicorcheas a 24 PPQN).
    pub ticks_per_step: u8,
    pub channel_mode: ChannelMode,
}

impl Pattern {
    /// Patrón inicial: 5 tracks de 16 pasos en canal 1, notas 60-64 (C4..E4).
    pub fn starter() -> Self {
        Self {
            tracks: (0..5).map(|i| Track::new(0, 60 + i, 16)).collect(),
            ticks_per_step: 6,
            channel_mode: ChannelMode::PerTrack,
        }
    }

    /// Notas que caen en `tick`. Única consulta que hace el motor durante la
    /// reproducción — reemplazar la grilla por una lista de eventos a futuro
    /// solo toca esta función.
    pub fn events_at(&self, tick: u32, out: &mut Vec<NoteEvent>) {
        let tps = self.ticks_per_step as u32;
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
