use esp_idf_svc::hal::delay::{FreeRtos, BLOCK};
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::uart::{config::Config as UartConfig, UartDriver};
use esp_idf_svc::hal::units::Hertz;

/// Note On fijo mandado por DIN OUT apenas arranca la tarea, para probar el
/// camino TX (GPIO17 → DIN OUT) sin depender de recibir nada por DIN IN.
/// Bytes elegidos para no confundirse con mensajes reales de prueba (nota y
/// velocidad 0x15 = 21, un valor que no suele usarse a mano).
const BOOT_TEST_MSG: [u8; 3] = [0x90, 21, 21];

/// DIN5 MIDI IN/OUT — prueba de loopback sobre UART2.
///
/// ## Conexionado del hardware MIDI a este devkit (ESP32-S3)
///
/// | Señal    | Pin ESP32-S3 | Notas |
/// |----------|--------------|-------|
/// | MIDI IN  | GPIO16 (RX)  | Va DESPUÉS del optoacoplador (6N137), no directo del DIN5. El opto aísla y ya deja la señal en niveles lógicos del micro. |
/// | MIDI OUT | GPIO17 (TX)  | Sale por las resistencias serie estándar (220 Ω) hacia los pines 4/5 del DIN5 hembra de salida. No lleva optoacoplador. |
/// | GND      | GND          | Común entre la electrónica MIDI y el ESP32 (el lado aislado del opto en IN no comparte GND — ver hoja de datos del 6N137). |
///
/// UART2 a 31250 bps, 8N1, sin control de flujo — el estándar MIDI. GPIO16/17
/// quedan libres en este devkit: no chocan con USB nativo (GPIO19/20, TinyUSB),
/// ni con el SPI de los encoders (GPIO4/5/10/18, ver `control_spi_encoders`).
///
/// ## Qué hace esta tarea (bring-up, no el firmware final)
///
/// Apenas arranca, manda un mensaje de prueba fijo por DIN OUT (sin depender
/// de haber recibido nada por DIN IN) — permite validar el camino TX
/// (GPIO17 → DIN OUT) de forma aislada del RX. Después entra en el loop de
/// siempre: bloquea esperando un mensaje MIDI de 3 bytes por DIN IN, lo
/// loguea por la consola (UART0, cable USB-UART separado), espera 1 segundo
/// y lo reenvía tal cual por DIN OUT. Sirve para validar el cableado extremo
/// a extremo antes de integrar `obrero-core` (parseo real de MIDI, running
/// status, mensajes de 1/2 bytes, etc. quedan para cuando esto se conecte al
/// motor).
pub unsafe extern "C" fn midi_din(_: *mut core::ffi::c_void) {
    let peripherals = Peripherals::take().unwrap();

    let config = UartConfig::new().baudrate(Hertz(31_250));
    let uart = UartDriver::new(
        peripherals.uart2,
        peripherals.pins.gpio17, // TX → MIDI OUT
        peripherals.pins.gpio16, // RX ← MIDI IN
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &config,
    )
    .unwrap();

    // Punto de sincronización para el harness de HIL: confirma que el UART
    // de DIN quedó configurado y la tarea está lista para recibir, sin
    // depender de esperar al primer mensaje MIDI.
    //
    // `eprintln!` (stderr) y no `println!` (stdout): stdout queda bufferizado
    // y nunca flushea en este target dentro de un programa que no termina —
    // nada impreso con `println!` llega jamás a la consola.
    eprintln!("[midi_din] listo");

    // Delay corto para darle tiempo al harness de HIL a terminar de abrir el
    // puerto MIDI IN del lado del host después de ver "listo" en el log —
    // si no, este mensaje puede llegar al driver USB-MIDI del host antes de
    // que la conexión esté armada y perderse sin que nadie lo vea.
    FreeRtos::delay_ms(500);
    match uart.write(&BOOT_TEST_MSG) {
        Ok(_) => eprintln!("[midi_din] OUT boot {:02X?}", BOOT_TEST_MSG),
        Err(e) => eprintln!("[midi_din] error al escribir mensaje de arranque: {:?}", e),
    }

    let mut buf = [0u8; 3];

    loop {
        match uart.read(&mut buf, BLOCK) {
            Ok(n) if n > 0 => {
                eprintln!("[midi_din] IN  {:02X?}", &buf[..n]);

                FreeRtos::delay_ms(1000);

                match uart.write(&buf[..n]) {
                    Ok(_) => eprintln!("[midi_din] OUT {:02X?}", &buf[..n]),
                    Err(e) => eprintln!("[midi_din] error al escribir: {:?}", e),
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("[midi_din] error de lectura: {:?}", e),
        }
    }
}
