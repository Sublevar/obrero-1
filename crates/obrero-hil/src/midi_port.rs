use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, bail, Result};
use midir::{MidiIO, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

/// Un puerto MIDI de entrada abierto contra el nombre indicado.
///
/// Los bytes recibidos se acumulan en un canal desde el hilo de callback de
/// `midir`; `recv_timeout` los va consumiendo desde el hilo del test.
pub struct MidiIn {
    _conn: MidiInputConnection<()>,
    rx: Receiver<(SystemTime, Vec<u8>)>,
}

impl MidiIn {
    pub fn open_matching(name_substr: &str) -> Result<Self> {
        let midi_in = MidiInput::new("obrero-hil")?;
        let port = find_port(&midi_in, name_substr)?;
        let (tx, rx) = mpsc::channel();
        let conn = midi_in
            .connect(
                &port,
                "obrero-hil-in",
                move |_stamp_us, bytes, _| {
                    let _ = tx.send((SystemTime::now(), bytes.to_vec()));
                },
                (),
            )
            .map_err(|e| anyhow!("conectando entrada MIDI: {e}"))?;
        Ok(Self { _conn: conn, rx })
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Option<(SystemTime, Vec<u8>)> {
        self.rx.recv_timeout(timeout).ok()
    }
}

pub struct MidiOut {
    conn: MidiOutputConnection,
}

impl MidiOut {
    pub fn open_matching(name_substr: &str) -> Result<Self> {
        let midi_out = MidiOutput::new("obrero-hil")?;
        let port = find_port(&midi_out, name_substr)?;
        let conn = midi_out
            .connect(&port, "obrero-hil-out")
            .map_err(|e| anyhow!("conectando salida MIDI: {e}"))?;
        Ok(Self { conn })
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        self.conn
            .send(bytes)
            .map_err(|e| anyhow!("enviando mensaje MIDI: {e}"))
    }
}

fn find_port<IO: MidiIO>(io: &IO, name_substr: &str) -> Result<IO::Port> {
    let ports = io.ports();
    for port in &ports {
        if let Ok(name) = io.port_name(port) {
            if name.contains(name_substr) {
                return Ok(port.clone());
            }
        }
    }
    let available: Vec<String> = ports.iter().filter_map(|p| io.port_name(p).ok()).collect();
    bail!(
        "ningún puerto MIDI contiene '{name_substr}'. Puertos disponibles: {:?}. \
         Corré `cargo run -p obrero-hil --bin obrero-hil-list-ports` para verlos.",
        available
    )
}
