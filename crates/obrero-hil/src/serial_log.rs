use std::io::{ErrorKind, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{bail, Context, Result};

#[derive(Clone, Debug)]
pub struct LogLine {
    pub at: SystemTime,
    pub text: String,
}

/// Lee el conector UART (consola de la placa, `println!` del firmware) en un
/// hilo de fondo y guarda las líneas recibidas, para poder esperar una línea
/// puntual (`wait_for`) o volcar todo lo visto hasta ahora cuando un test
/// falla (`dump`).
///
/// El puerto serie en sí vive DENTRO del hilo de fondo, no en este struct —
/// por eso `Drop` avisa al hilo (`stop`) y lo espera (`join`): sin eso, el
/// puerto queda abierto colgado en un `read()` con timeout hasta que a la
/// placa se le ocurra mandar otra línea, y el próximo test que intente
/// flashear el mismo puerto se encuentra con "Device or resource busy"
/// aunque el `Harness` anterior ya se haya dropeado.
pub struct SerialLog {
    rx: Receiver<LogLine>,
    buffer: Vec<LogLine>,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
}

impl SerialLog {
    pub fn open(port: &str, baud: u32) -> Result<Self> {
        let mut serial = serialport::new(port, baud)
            .timeout(Duration::from_millis(200))
            .dtr_on_open(false)
            .open()
            .with_context(|| format!("abriendo puerto de log UART {port}"))?;

        // Abrir el puerto igual pulsa DTR un instante (la doc de `serialport`
        // lo advierte — no se puede evitar del todo), y en los devkits con
        // auto-reset eso puede dejar el chip colgado en el bootloader ROM en
        // vez de correr la app. Liberar ambas líneas apenas se abre (sin
        // generar nosotros un pulso adicional — probado en banco: pulsar RTS
        // acá metía al chip en modo bootloader en vez de dejarlo correr) deja
        // que el reset que ya hizo `espflash` al terminar de flashear resuelva
        // en un arranque normal.
        serial.write_data_terminal_ready(false).ok();
        serial.write_request_to_send(false).ok();

        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_reader = stop.clone();
        let reader = std::thread::spawn(move || {
            let mut byte = [0u8; 1];
            let mut line = Vec::new();
            while !stop_reader.load(Ordering::Relaxed) {
                match serial.read(&mut byte) {
                    Ok(0) => break,
                    Ok(_) => {
                        if byte[0] == b'\n' {
                            let text = String::from_utf8_lossy(&line).trim_end().to_string();
                            line.clear();
                            if tx
                                .send(LogLine {
                                    at: SystemTime::now(),
                                    text,
                                })
                                .is_err()
                            {
                                break;
                            }
                        } else {
                            line.push(byte[0]);
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::TimedOut => continue,
                    Err(_) => break,
                }
            }
            // `serial` se dropea acá al salir del closure, cerrando el puerto.
        });

        Ok(Self {
            rx,
            buffer: Vec::new(),
            stop,
            reader: Some(reader),
        })
    }

    /// Vuelca al buffer interno cualquier línea recibida desde el último poll.
    pub fn poll(&mut self) {
        while let Ok(line) = self.rx.try_recv() {
            self.buffer.push(line);
        }
    }

    /// Bloquea hasta ver una línea que cumpla `pred`, o hasta agotar `timeout`.
    pub fn wait_for(&mut self, timeout: Duration, pred: impl Fn(&str) -> bool) -> Result<LogLine> {
        let deadline = Instant::now() + timeout;
        loop {
            self.poll();
            if let Some(line) = self.buffer.iter().find(|l| pred(&l.text)) {
                return Ok(line.clone());
            }
            if Instant::now() >= deadline {
                bail!("timeout de {timeout:?} esperando una línea de log que cumpla la condición");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Imprime todo lo visto hasta ahora — pensado para llamarse cuando un
    /// assert falla, para tener el log de la placa correlacionado a mano.
    pub fn dump(&self) {
        for line in &self.buffer {
            eprintln!("[uart] {}", line.text);
        }
    }
}

impl Drop for SerialLog {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Esperar a que el hilo realmente salga (y dropee el puerto) antes de
        // devolver el control — si no, el próximo test puede intentar abrir
        // el mismo puerto mientras este hilo todavía lo tiene tomado.
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
