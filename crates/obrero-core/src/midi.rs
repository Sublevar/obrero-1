enum ParseState {
    Idle,
    Data1 { status: u8 },
    Data2 { status: u8, d1: u8 },
}

pub struct MidiParser {
    state: ParseState,
}

impl MidiParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::Idle,
        }
    }

    /// Feed one raw byte. Returns Some((status, d1, d2)) when a complete message is ready.
    pub fn feed(&mut self, byte: u8) -> Option<(u8, u8, u8)> {
        if byte & 0x80 != 0 {
            // System Real-Time (0xF8–0xFF): single-byte, don't interrupt running status
            if byte >= 0xF8 {
                return Some((byte, 0, 0));
            }
            match data_bytes_for(byte) {
                0 => {
                    self.state = ParseState::Idle;
                    return Some((byte, 0, 0));
                }
                1 => self.state = ParseState::Data1 { status: byte },
                _ => self.state = ParseState::Data1 { status: byte },
            }
        } else {
            match self.state {
                ParseState::Data1 { status } => {
                    if data_bytes_for(status) == 1 {
                        // running status: keep state, emit message
                        return Some((status, byte, 0));
                    } else {
                        self.state = ParseState::Data2 { status, d1: byte };
                    }
                }
                ParseState::Data2 { status, d1 } => {
                    self.state = ParseState::Data1 { status }; // running status
                    return Some((status, d1, byte));
                }
                ParseState::Idle => {} // stray data byte — discard
            }
        }
        None
    }
}

fn data_bytes_for(status: u8) -> u8 {
    match status & 0xF0 {
        0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0 => 2,
        0xC0 | 0xD0 => 1,
        0xF0 => match status {
            0xF1 | 0xF3 => 1,
            0xF2 => 2,
            _ => 0,
        },
        _ => 0,
    }
}

impl Default for MidiParser {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_midi_msg(status: u8, d1: u8, d2: u8) -> std::string::String {
    let ch = (status & 0x0F) + 1;
    match status & 0xF0 {
        0x80 => format!("Note Off    ch={:2} note={:3} vel={:3}", ch, d1, d2),
        0x90 => {
            if d2 == 0 {
                format!(
                    "Note Off    ch={:2} note={:3} (vel=0 via running status)",
                    ch, d1
                )
            } else {
                format!("Note On     ch={:2} note={:3} vel={:3}", ch, d1, d2)
            }
        }
        0xA0 => format!("Aftertouch  ch={:2} note={:3} val={:3}", ch, d1, d2),
        0xB0 => format!("CC          ch={:2} cc={:3}  val={:3}", ch, d1, d2),
        0xC0 => format!("ProgChange  ch={:2} prog={:3}", ch, d1),
        0xD0 => format!("ChanPress   ch={:2} val={:3}", ch, d1),
        0xE0 => {
            let bend = (((d2 as i16) << 7) | d1 as i16) - 8192;
            format!("PitchBend   ch={:2} val={:6}", ch, bend)
        }
        0xF0 => match status {
            0xF0 => "SysEx Start".to_string(),
            0xF1 => format!("MTC Quarter Frame  val={}", d1),
            0xF2 => format!("Song Pos           lsb={} msb={}", d1, d2),
            0xF3 => format!("Song Select        song={}", d1),
            0xF6 => "Tune Request".to_string(),
            0xF7 => "SysEx End".to_string(),
            0xF8 => "Clock".to_string(),
            0xFA => "Start".to_string(),
            0xFB => "Continue".to_string(),
            0xFC => "Stop".to_string(),
            0xFE => "Active Sense".to_string(),
            0xFF => "Reset".to_string(),
            _ => format!("SysCommon 0x{:02X}", status),
        },
        _ => format!("Unknown 0x{:02X} {:02X} {:02X}", status, d1, d2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(bytes: &[u8]) -> Vec<(u8, u8, u8)> {
        let mut p = MidiParser::new();
        bytes.iter().filter_map(|&b| p.feed(b)).collect()
    }

    #[test]
    fn note_on_three_bytes() {
        assert_eq!(parse_all(&[0x90, 60, 100]), vec![(0x90, 60, 100)]);
    }

    #[test]
    fn running_status_two_byte_messages() {
        // Note On ch1 + dos mensajes más sin repetir el status
        let msgs = parse_all(&[0x90, 60, 100, 62, 101, 64, 102]);
        assert_eq!(
            msgs,
            vec![(0x90, 60, 100), (0x90, 62, 101), (0x90, 64, 102)]
        );
    }

    #[test]
    fn running_status_one_data_byte() {
        // Program Change usa 1 byte de dato
        let msgs = parse_all(&[0xC3, 10, 11]);
        assert_eq!(msgs, vec![(0xC3, 10, 0), (0xC3, 11, 0)]);
    }

    #[test]
    fn realtime_byte_does_not_break_running_status() {
        // 0xF8 (Clock) intercalado en medio de un Note On
        let msgs = parse_all(&[0x90, 60, 0xF8, 100]);
        assert_eq!(msgs, vec![(0xF8, 0, 0), (0x90, 60, 100)]);
    }

    #[test]
    fn stray_data_bytes_discarded() {
        assert_eq!(parse_all(&[42, 43, 44]), vec![]);
    }

    #[test]
    fn new_status_resets_partial_message() {
        // Note On incompleto interrumpido por otro Note On
        let msgs = parse_all(&[0x90, 60, 0x91, 70, 90]);
        assert_eq!(msgs, vec![(0x91, 70, 90)]);
    }

    #[test]
    fn system_common_messages() {
        // Song Position (2 datos), Song Select (1 dato), Tune Request (0 datos)
        let msgs = parse_all(&[0xF2, 0x01, 0x02, 0xF3, 5, 0xF6]);
        assert_eq!(msgs, vec![(0xF2, 0x01, 0x02), (0xF3, 5, 0), (0xF6, 0, 0)]);
    }

    #[test]
    fn transport_realtime_bytes() {
        let msgs = parse_all(&[0xFA, 0xF8, 0xFC, 0xFB]);
        assert_eq!(
            msgs,
            vec![(0xFA, 0, 0), (0xF8, 0, 0), (0xFC, 0, 0), (0xFB, 0, 0)]
        );
    }
}
