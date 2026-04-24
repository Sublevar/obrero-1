/// Encoder rotativo con botón central.
///
/// Cada encoder ocupa 3 bits en el buffer SPI:
///   bit 0 (LSB del grupo): horario   (CW)
///   bit 1                : botón     (BTN)
///   bit 2                : antihorario (CCW)
///
/// Los 8 encoders se construyen desde el LSB del valor de 24 bits:
///   encoder 1 → bits [0..2], encoder 2 → bits [3..5], ..., encoder 8 → bits [21..23]
pub struct Encoder {
    pub turns: i32,
    prev_cw: bool,
    prev_ccw: bool,
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
            turns: 0,
            prev_cw: false,
            prev_ccw: false,
        }
    }

    /// Actualiza el estado con los tres bits del ciclo actual y llama al
    /// callback si se detectó un flanco (giro o botón pulsado).
    ///
    /// `callback(turns: i32, btn: bool)`
    pub fn update_with<F>(&mut self, cw: bool, btn: bool, ccw: bool, mut callback: F)
    where
        F: FnMut(i32, bool),
    {
        let cw_edge  = cw  && !self.prev_cw;
        let ccw_edge = ccw && !self.prev_ccw;

        if cw_edge  { self.turns += 1; }
        if ccw_edge { self.turns -= 1; }

        if cw_edge || ccw_edge || btn {
            callback(self.turns, btn);
        }

        self.prev_cw  = cw;
        self.prev_ccw = ccw;
    }
}

/// Construye un array de 8 encoders a partir del valor de 24 bits leído por SPI.
///
/// Llamar en cada ciclo de lectura; internamente detecta flancos con el estado previo.
pub fn update_encoders_from_bits(encoders: &mut [Encoder; 8], bits: u32) {
    for i in 0..8usize {
        let base = i * 3;
        let cw  = (bits >>  base)      & 1 == 1;
        let btn = (bits >> (base + 1)) & 1 == 1;
        let ccw = (bits >> (base + 2)) & 1 == 1;

        encoders[i].update_with(cw, btn, ccw, |turns, pressed| {
            println!(
                "[enc{}] turns={:+}  btn={}  cw={}  ccw={}",
                i + 1, turns, pressed, cw, ccw
            );
        });
    }
}
