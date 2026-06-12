# obrero-1

ESP32 Rust demo usando FreeRTOS tasks via `esp-idf-svc`.


# Desarrollo hardware
## circuitos Encoders:
el pin horario y antiorario van PULL UP
el pin del boton va a PULL DOWN

# desarrollo software
## Dependencias

| Crate | Version | Uso |
|---|---|---|
| `esp-idf-svc` | 0.51 | HAL + servicios ESP-IDF (GPIO, timers, FreeRTOS) |
| `log` | 0.4 | Facade de logging |
| `anyhow` | 1.0 | Manejo de errores |
| `embuild` | 0.33 | Descarga y linking de ESP-IDF en tiempo de build |

**Target:** `xtensa-esp32-espidf`
**ESP-IDF:** v5.3.3
**Toolchain:** `esp` (fork Xtensa de Rust — ver `rust-toolchain.toml`)

## Setup

```sh
# Instalar toolchain ESP Rust
cargo install espup && espup install
source ~/export-esp.sh   # agregar al perfil de shell

# Instalar herramienta de flasheo
cargo install espflash
```

## Conectores de entrada (molex_PCB)

El circuito expone dos conectores Molex de 5 pines en la PCB.

### Conector 1 — entrada de datos serie (DS)

| Pin | Señal | GPIO ESP32 | Descripción |
|-----|-------|------------|-------------|
| 1   | VCC   | —          | Alimentación |
| 2   | PL    | `gpio22`   | Parallel Load / SH-!LD (latch de entradas) |
| 3   | CP    | `gpio23`   | Clock Pulse / SPI2 SCLK |
| 4   | DS    | `gpio2`    | Data Serial in / SPI2 MOSI (dummy, no conectado) |
| 5   | GND   | —          | Tierra |

### Conector 2 — salida de datos serie (Q7)

| Pin | Señal | GPIO ESP32 | Descripción |
|-----|-------|------------|-------------|
| 1   | VCC   | —          | Alimentación |
| 2   | PL    | `gpio22`   | Parallel Load / SH-!LD (latch de entradas) |
| 3   | CP    | `gpio23`   | Clock Pulse / SPI2 SCLK |
| 4   | Q7    | `gpio35`   | Salida serie del último shift register / SPI2 MISO |
| 5   | GND   | —          | Tierra |

## Pines ESP32 usados

- `gpio22` → LC / SH-!LD latch (pulso LOW para capturar entradas, luego HIGH)
- `gpio23` → SPI2 SCLK
- `gpio2`  → SPI2 MOSI (dummy, no conectado)
- `gpio35` → SPI2 MISO (QH salida serie)

## Configuración de encoders

El número de encoders se define en el archivo `.env` de la raíz del proyecto.

- Variable: `ENCODER_COUNT`
- Valor válido: `1` a `8`
- Ejemplo para probar solo 4 encoders:

```env
ENCODER_COUNT=4
```

Si no se define, el valor por defecto es `8`.

La primera build descarga ESP-IDF y el toolchain Xtensa en `.embuild/` — tarda varios minutos. Las builds siguientes son rápidas.

## Workflow

```sh
# Build
cargo build

# Build + flash + monitor serial (todo junto)
./dev.sh

# Flash + monitor manualmente
espflash flash target/xtensa-esp32-espidf/debug/obrero-1
espflash monitor
```

## Entorno de testeo

[Web MIDI Tester](https://studiocode.dev/webmidi-tester/midi)