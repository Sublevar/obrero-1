//! Harness de hardware-in-the-loop para probar el firmware ESP32 contra
//! una placa real conectada por MIDI DIN5. Ver el README de este crate para
//! el armado del banco de pruebas y las variables de entorno requeridas.

mod flasher;
mod harness;
mod midi_port;
mod serial_log;

pub use harness::{Config, Harness};
pub use midi_port::{MidiIn, MidiOut};
pub use serial_log::{LogLine, SerialLog};
