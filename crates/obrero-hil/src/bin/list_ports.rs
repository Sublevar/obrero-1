//! Lista los puertos MIDI y serie disponibles, para configurar
//! OBRERO_HIL_MIDI_IN / OBRERO_HIL_MIDI_OUT / OBRERO_HIL_UART_PORT.
//! Uso: cargo run -p obrero-hil --bin obrero-hil-list-ports

fn main() -> anyhow::Result<()> {
    println!("== Puertos MIDI IN ==");
    let midi_in = midir::MidiInput::new("obrero-hil-list-ports")?;
    for port in midi_in.ports() {
        println!("  {}", midi_in.port_name(&port)?);
    }

    println!("== Puertos MIDI OUT ==");
    let midi_out = midir::MidiOutput::new("obrero-hil-list-ports")?;
    for port in midi_out.ports() {
        println!("  {}", midi_out.port_name(&port)?);
    }

    println!("== Puertos serie ==");
    for port in serialport::available_ports()? {
        println!("  {} ({:?})", port.port_name, port.port_type);
    }

    Ok(())
}
