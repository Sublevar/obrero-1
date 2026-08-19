/// Eventos de entrada de la UI física (botones/encoder) o web.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Play,
    Stop,
    Shift,
    EncoderPush,
    /// Botón de paso 0-15.
    Step(u8),
    /// Selección de track 0-N.
    TrackSelect(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    ButtonDown(Button),
    ButtonUp(Button),
    EncoderDelta(i32),
}
