use std::env;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::flasher;
use crate::midi_port::{MidiIn, MidiOut};
use crate::serial_log::SerialLog;

const DEFAULT_UART_BAUD: u32 = 115_200; // consola ESP-IDF por defecto (CONFIG_ESP_CONSOLE_UART_DEFAULT)
const DEFAULT_BOOT_MARKER: &str = "[midi_din] listo";

pub struct Config {
    pub firmware_dir: PathBuf,
    pub uart_port: Option<String>,
    pub uart_baud: u32,
    pub midi_in_name: String,
    pub midi_out_name: String,
    pub boot_marker: String,
}

impl Config {
    /// Lee la configuración del rig desde variables de entorno. Ver el
    /// README de este crate para el detalle de cada una.
    pub fn from_env() -> Result<Self> {
        if env::var("OBRERO_HIL").as_deref() != Ok("1") {
            bail!(
                "estos son tests de hardware-in-the-loop: necesitan una placa conectada. \
                 Corré con OBRERO_HIL=1 y las variables OBRERO_HIL_MIDI_IN/OBRERO_HIL_MIDI_OUT \
                 seteadas — ver crates/obrero-hil/README.md"
            );
        }

        let midi_in_name = env::var("OBRERO_HIL_MIDI_IN")
            .context("falta OBRERO_HIL_MIDI_IN (substring del nombre del puerto MIDI IN)")?;
        let midi_out_name = env::var("OBRERO_HIL_MIDI_OUT")
            .context("falta OBRERO_HIL_MIDI_OUT (substring del nombre del puerto MIDI OUT)")?;

        let firmware_dir = match env::var("OBRERO_HIL_FIRMWARE_DIR") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../firmware")
                .canonicalize()
                .context("no se encontró firmware/ — seteá OBRERO_HIL_FIRMWARE_DIR")?,
        };

        let uart_baud = match env::var("OBRERO_HIL_UART_BAUD") {
            Ok(v) => v.parse().context("OBRERO_HIL_UART_BAUD inválido")?,
            Err(_) => DEFAULT_UART_BAUD,
        };

        Ok(Self {
            firmware_dir,
            uart_port: env::var("OBRERO_HIL_UART_PORT").ok(),
            uart_baud,
            midi_in_name,
            midi_out_name,
            boot_marker: env::var("OBRERO_HIL_BOOT_MARKER")
                .unwrap_or_else(|_| DEFAULT_BOOT_MARKER.to_string()),
        })
    }

    fn resolve_uart_port(&self) -> Result<String> {
        if let Some(port) = &self.uart_port {
            return Ok(port.clone());
        }
        autodetect_uart_port()
    }
}

/// Busca un puerto serie USB entre los disponibles (el conector UART del
/// devkit, no el puerto MIDI). A diferencia de `dev.sh` (que asume
/// `/dev/ttyUSB*`, Linux), usa `serialport::available_ports` para no
/// depender de convenciones de path por sistema operativo.
fn autodetect_uart_port() -> Result<String> {
    let ports = serialport::available_ports().context("listando puertos serie")?;
    let usb_port = ports
        .into_iter()
        .find(|p| matches!(p.port_type, serialport::SerialPortType::UsbPort(_)));
    match usb_port {
        Some(p) => Ok(p.port_name),
        None => bail!(
            "no encontré ningún puerto serie USB — conectá el cable UART del devkit \
             o seteá OBRERO_HIL_UART_PORT"
        ),
    }
}

/// Cada `Harness` habla con el ÚNICO banco físico conectado (una placa, un
/// puerto UART, una interfaz MIDI). `cargo test` corre los tests de un mismo
/// binario en threads paralelos por default, así que sin este lock dos tests
/// HIL intentan flashear el mismo puerto UART a la vez y truenan con
/// "Device or resource busy". Se toma en `flash_and_boot` y se mantiene
/// mientras el `Harness` esté vivo, serializando los tests automáticamente
/// — no hace falta acordarse de correr con `--test-threads=1`.
static HIL_LOCK: Mutex<()> = Mutex::new(());

/// Handle sobre una placa flasheada y lista: puertos MIDI abiertos y el
/// log de UART0 capturándose en un hilo de fondo.
///
/// Orden de campos a propósito: Rust dropea en orden de declaración, y acá
/// necesitamos que `log` (que cierra el puerto UART en su propio `Drop`, ver
/// `SerialLog`) termine ANTES de soltar `_lock` — si no, el próximo test
/// puede intentar flashear el mismo puerto mientras el hilo lector de este
/// `Harness` todavía lo tiene abierto.
pub struct Harness {
    midi_in: MidiIn,
    midi_out: MidiOut,
    log: SerialLog,
    _lock: MutexGuard<'static, ()>,
}

impl Harness {
    /// Compila+flashea el firmware, espera a que reporte estar listo, y
    /// abre los puertos MIDI configurados. Bloquea si otro `Harness` ya
    /// está usando el banco (ver `HIL_LOCK`).
    pub fn flash_and_boot(config: &Config) -> Result<Self> {
        let lock = HIL_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let uart_port = config.resolve_uart_port()?;

        flasher::build_and_flash(&config.firmware_dir, &uart_port)?;

        let mut log = SerialLog::open(&uart_port, config.uart_baud)?;
        log.wait_for(Duration::from_secs(15), |line| {
            line.contains(&config.boot_marker)
        })
        .context("la placa no reportó estar lista después de flashear")?;

        let midi_in = MidiIn::open_matching(&config.midi_in_name)?;
        let midi_out = MidiOut::open_matching(&config.midi_out_name)?;

        Ok(Self {
            midi_in,
            midi_out,
            log,
            _lock: lock,
        })
    }

    pub fn send_midi(&mut self, bytes: &[u8]) -> Result<()> {
        self.midi_out.send(bytes)
    }

    /// Bloquea hasta recibir por MIDI IN un mensaje que cumpla `pred`, o
    /// hasta agotar `timeout`. Al fallar, vuelca el log de UART0 acumulado
    /// para poder correlacionar qué vio la placa mientras tanto.
    pub fn expect_midi_within(
        &mut self,
        timeout: Duration,
        pred: impl Fn(&[u8]) -> bool,
    ) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        loop {
            self.log.poll();

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.log.dump();
                bail!("timeout de {timeout:?} esperando un mensaje MIDI que cumpla la condición");
            }

            let wait = remaining.min(Duration::from_millis(50));
            if let Some((_, bytes)) = self.midi_in.recv_timeout(wait) {
                if pred(&bytes) {
                    return Ok(bytes);
                }
            }
        }
    }

    /// Bloquea hasta ver por UART0 una línea de log que cumpla `pred`, o
    /// hasta agotar `timeout`. Al fallar, vuelca el log acumulado.
    pub fn expect_log_within(
        &mut self,
        timeout: Duration,
        pred: impl Fn(&str) -> bool,
    ) -> Result<String> {
        self.log
            .wait_for(timeout, pred)
            .inspect_err(|_| self.log.dump())
            .map(|line| line.text)
    }

    /// Vuelca a stderr todo el log de UART0 visto hasta ahora.
    pub fn dump_log(&mut self) {
        self.log.poll();
        self.log.dump();
    }
}
