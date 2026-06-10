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

#[derive(Clone, Debug)]
pub struct Track {
    /// Canal MIDI 0-15.
    pub channel: u8,
    pub steps: Vec<Step>,
    /// Largo activo del track en pasos (permite polimetría; <= steps.len()).
    pub len: usize,
    pub muted: bool,
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
        }
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
    /// Patrón inicial: 4 tracks de batería GM (canal 10) de 16 pasos.
    pub fn default_drums() -> Self {
        Self {
            tracks: vec![
                Track::new(9, 36, 16), // bombo
                Track::new(9, 38, 16), // redoblante
                Track::new(9, 42, 16), // hi-hat cerrado
                Track::new(9, 46, 16), // hi-hat abierto
            ],
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
