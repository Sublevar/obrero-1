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