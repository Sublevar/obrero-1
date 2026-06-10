/// Estado de la UI que produce el motor. El core no dibuja nada: el firmware
/// lo renderiza en el display y la web lo serializa hacia JS.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Debug)]
pub struct StepView {
    pub on: bool,
    pub note: u8,
    pub velocity: u8,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Debug)]
pub struct TrackView {
    pub channel: u8,
    pub muted: bool,
    /// Nota base del track (lane drum machine: la de sus pasos).
    pub note: u8,
    pub steps: Vec<StepView>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Debug)]
pub struct ViewModel {
    pub playing: bool,
    pub bpm: f32,
    pub external_clock: bool,
    /// Some(canal) = modo canal único (drum machine); None = canal por track.
    pub single_channel: Option<u8>,
    /// Paso que está sonando (índice sobre el track seleccionado).
    pub current_step: usize,
    pub selected_track: usize,
    pub tracks: Vec<TrackView>,
}
