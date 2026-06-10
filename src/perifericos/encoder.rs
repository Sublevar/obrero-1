/// Encoder rotativo con botón central.
///
/// Hardware: 2× 74HC165 en cadena, desplazando QH primero (MSB del byte).
/// Cada IC aloja 2 encoders en pines B-G; pines A (IC pin 11) y H (IC pin 6) sin conectar.
///
/// Layout de pines por IC (A=bit0 ... H=bit7):
///   bit0=A(NC) bit1=B(BTN) bit2=C(CW) bit3=D(CCW) bit4=E(CW) bit5=F(CCW) bit6=G(BTN) bit7=H(NC)
///
/// Posiciones en el valor 16-bit  (bits = buf[1]<<8 | buf[0]):
///   enc1 (chip1 B/C/D): CW=bit2,  CCW=bit3,  BTN=bit1
///   enc2 (chip1 E/F/G): CW=bit4,  CCW=bit5,  BTN=bit6
///   enc3 (chip2 B/C/D): CW=bit10, CCW=bit11, BTN=bit9
///   enc4 (chip2 E/F/G): CW=bit12, CCW=bit13, BTN=bit14
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

/// Si es `false`, los eventos de sólo-botón no se imprimen (rotación siempre se imprime).
pub const DEBUG_BTN: bool = false;

/// Actualiza los encoders desde el valor de 16 bits leído por SPI (2× 74HC165).
///
/// Ver doc del struct para el layout de pines y posiciones de bits.
/// Llamar en cada ciclo de lectura; internamente detecta flancos con el estado previo.
pub fn update_encoders_from_bits(encoders: &mut [Encoder], bits: u32) {
    // (cw_bit, ccw_bit, btn_bit) — ver layout en doc del módulo
    const LAYOUT: [(u32, u32, u32); 4] = [
        ( 2,  3,  1), // enc1: chip1 C(CW)/D(CCW)/B(BTN)
        ( 4,  5,  6), // enc2: chip1 E(CW)/F(CCW)/G(BTN)
        (10, 11,  9), // enc3: chip2 C(CW)/D(CCW)/B(BTN)
        (12, 13, 14), // enc4: chip2 E(CW)/F(CCW)/G(BTN)
    ];

    for (i, &(cw_bit, ccw_bit, btn_bit)) in LAYOUT.iter().enumerate().take(encoders.len()) {
        let cw  = (bits >> cw_bit)  & 1 == 1;
        let ccw = (bits >> ccw_bit) & 1 == 1;
        let btn = (bits >> btn_bit) & 1 == 1;

        encoders[i].update_with(cw, btn, ccw, |turns, pressed| {
            if !pressed || DEBUG_BTN {
                println!(
                    "[enc{}] turns={:+}  btn={}  cw={}  ccw={}",
                    i + 1, turns, pressed, cw, ccw
                );
            }
        });
    }
}
