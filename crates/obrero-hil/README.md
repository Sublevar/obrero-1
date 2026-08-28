# obrero-hil

Tests de hardware-in-the-loop (HIL) para el firmware ESP32: flashean la placa
de verdad, le hablan MIDI por DIN5 a través de una interfaz USB-MIDI externa,
y verifican los mensajes que devuelve (más el log de UART0 en paralelo, para
poder correlacionar qué vio la placa cuando algo falla).

Corren `#[ignore]` por defecto — necesitan hardware conectado — y quedan
fuera de la corrida normal de CI (`cargo test --workspace`).

## Banco de pruebas

```
Host ──USB (flash/monitor)── ESP32-S3 devkit ──DIN5── interfaz USB-MIDI ──USB── Host
```

- El cable UART del devkit (bridge USB-UART) para flashear y leer la consola.
- Las salidas/entradas DIN5 de la placa (ver `firmware/src/tasks/midi_din.rs`
  para el pinout: GPIO16=RX/MIDI IN, GPIO17=TX/MIDI OUT, UART2 a 31250 bps)
  cableadas a una interfaz USB-MIDI class-compliant cualquiera.
- La interfaz USB-MIDI conectada al mismo host, aparece como un puerto MIDI
  normal (CoreMIDI/ALSA/WinMM) que este crate usa vía `midir`.

## Setup

1. Toolchain xtensa activo en la shell (igual que para `firmware/dev.sh`):
   ```sh
   source ~/export-esp.sh
   ```
2. Encontrar los nombres de puerto:
   ```sh
   cargo run -p obrero-hil --bin obrero-hil-list-ports
   ```
3. Correr el test:
   ```sh
   OBRERO_HIL=1 \
   OBRERO_HIL_MIDI_IN="nombre o substring del puerto MIDI IN" \
   OBRERO_HIL_MIDI_OUT="nombre o substring del puerto MIDI OUT" \
   cargo test -p obrero-hil -- --ignored --nocapture
   ```

## Variables de entorno

| Variable | Requerida | Descripción |
|---|---|---|
| `OBRERO_HIL` | sí | Debe ser `1`. Gate explícito para no correr estos tests por accidente. |
| `OBRERO_HIL_MIDI_IN` | sí | Substring del nombre del puerto MIDI IN (recibe lo que la placa manda). |
| `OBRERO_HIL_MIDI_OUT` | sí | Substring del nombre del puerto MIDI OUT (por donde el harness le manda MIDI a la placa). |
| `OBRERO_HIL_UART_PORT` | no | Puerto serie del conector UART. Si no se setea, se autodetecta el primer puerto serie USB disponible. |
| `OBRERO_HIL_UART_BAUD` | no | Default 115200 (consola ESP-IDF). |
| `OBRERO_HIL_FIRMWARE_DIR` | no | Default `../../firmware` relativo a este crate. |
| `OBRERO_HIL_BOOT_MARKER` | no | Línea de log que indica que la placa está lista. Default `[midi_din] listo`. |

## Qué cubre hoy

Tres tests en `tests/din_loopback.rs`, todos sobre el mismo banco físico:

- `din_echoes_note_on_after_one_second`: manda un Note On por DIN IN y
  espera que la placa lo devuelva por DIN OUT ~1s después (el delay que hoy
  tiene `midi_din.rs` a propósito, para poder verificar a mano el cableado).
  Ejercita IN y OUT juntos.
- `din_note_on_is_logged_over_serial`: sólo la mitad IN — el mismo Note On
  tiene que verse logueado por UART0 casi al instante, sin esperar el eco.
- `din_out_reaches_host_on_boot`: sólo la mitad OUT, sin depender de DIN
  IN — la placa manda un mensaje de prueba fijo (`BOOT_TEST_MSG` en
  `midi_din.rs`) apenas arranca, y este test espera verlo llegar a la
  interfaz USB-MIDI del host. Útil para diagnosticar: si este pasa pero los
  otros dos no, el problema está del lado de DIN IN (opto/GPIO16), no de
  DIN OUT.

Sirven para validar el banco en sí — cableado, aislación óptica, baudrate,
matching de puertos — antes de construir escenarios más ambiciosos (patrón +
play) que dependen de que DIN se conecte a `obrero-core` como sink real, algo
que todavía no existe (ver `docs/product/usb-midi-hardware.md` §5/§7).
