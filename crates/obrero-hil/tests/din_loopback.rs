//! Test de hardware-in-the-loop: requiere una placa ESP32-S3 real, cableada
//! por MIDI DIN5 a una interfaz USB-MIDI, y el conector UART conectado.
//! Ver crates/obrero-hil/README.md para el armado del banco y las
//! variables de entorno necesarias.
//!
//! Corrida manual: OBRERO_HIL=1 ... cargo test -p obrero-hil -- --ignored

use std::time::Duration;

use obrero_hil::{Config, Harness};

#[test]
#[ignore = "requiere una placa ESP32-S3 conectada — ver crates/obrero-hil/README.md"]
fn din_echoes_note_on_after_one_second() {
    let config = Config::from_env().expect("configuración del rig (variables OBRERO_HIL_*)");
    let mut harness = Harness::flash_and_boot(&config).expect("flashear y arrancar la placa");

    let note_on = [0x90, 60, 100];
    let sent_at = std::time::Instant::now();
    harness.send_midi(&note_on).expect("enviar MIDI IN");

    // `expect_midi_within` ya vuelca el log de UART0 acumulado si falla — no
    // hace falta un `dump_log()` manual acá también.
    let echoed = harness
        .expect_midi_within(Duration::from_secs(3), |bytes| bytes == note_on)
        .unwrap_or_else(|e| panic!("la placa no devolvió el mensaje: {e}"));
    assert_eq!(echoed, note_on);

    // La tarea firmware espera 1s antes de reenviar (ver
    // firmware/src/tasks/midi_din.rs) — si vuelve antes, algo cambió el
    // comportamiento esperado.
    let elapsed = sent_at.elapsed();
    assert!(
        elapsed >= Duration::from_millis(950),
        "el eco llegó demasiado rápido ({elapsed:?}) para el delay de 1s de la tarea"
    );
}

/// Cubre sólo la mitad IN del camino: un mensaje mandado por DIN IN tiene que
/// verse logueado por la consola (UART0) casi al instante — sin esperar el
/// delay de 1s del reenvío por DIN OUT que sí ejercita `din_echoes_note_on_after_one_second`.
#[test]
#[ignore = "requiere una placa ESP32-S3 conectada — ver crates/obrero-hil/README.md"]
fn din_note_on_is_logged_over_serial() {
    let config = Config::from_env().expect("configuración del rig (variables OBRERO_HIL_*)");
    let mut harness = Harness::flash_and_boot(&config).expect("flashear y arrancar la placa");

    let note_on = [0x90, 64, 90];
    harness.send_midi(&note_on).expect("enviar MIDI IN");

    // Mismo formato que usa `println!("[midi_din] IN  {:02X?}", ...)` en el
    // firmware (ver firmware/src/tasks/midi_din.rs) — ej. "[90, 40, 5A]".
    let expected_bytes = format!("{note_on:02X?}");
    let line = harness
        .expect_log_within(Duration::from_secs(2), |line| {
            line.contains("[midi_din] IN") && line.contains(&expected_bytes)
        })
        .unwrap_or_else(|e| panic!("el mensaje no se vio logueado por UART0: {e}"));

    assert!(
        line.contains(&expected_bytes),
        "línea de log inesperada: {line}"
    );
}

/// Cubre sólo la mitad OUT del camino, sin depender de DIN IN: la placa
/// manda un mensaje de prueba fijo por DIN OUT apenas termina de arrancar
/// (ver `BOOT_TEST_MSG` en firmware/src/tasks/midi_din.rs), y este test
/// verifica que llega tal cual a la interfaz USB-MIDI del host. Sirve para
/// aislar el camino TX (GPIO17 → resistencias → DIN OUT) del RX — si este
/// test pasa pero `din_echoes_note_on_after_one_second` no, el problema está
/// del lado de DIN IN (opto/GPIO16), no de DIN OUT.
#[test]
#[ignore = "requiere una placa ESP32-S3 conectada — ver crates/obrero-hil/README.md"]
fn din_out_reaches_host_on_boot() {
    let config = Config::from_env().expect("configuración del rig (variables OBRERO_HIL_*)");
    let mut harness = Harness::flash_and_boot(&config).expect("flashear y arrancar la placa");

    let boot_test = [0x90, 21, 21];
    let received = harness
        .expect_midi_within(Duration::from_secs(5), |bytes| bytes == boot_test)
        .unwrap_or_else(|e| panic!("no llegó el mensaje de arranque por DIN OUT: {e}"));

    assert_eq!(received, boot_test);
}
