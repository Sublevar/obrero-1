# obrero-1

ESP32 Rust demo usando FreeRTOS tasks via `esp-idf-svc`.

## Dependencias

| Crate | Version | Uso |
|---|---|---|
| `esp-idf-svc` | 0.51 | HAL + servicios ESP-IDF (GPIO, timers, FreeRTOS) |
| `log` | 0.4 | Facade de logging |
| `anyhow` | 1.0 | Manejo de errores |
| `embuild` | 0.33 | Descarga y linking de ESP-IDF en tiempo de build |

**Target activo:** ver `.cargo/config.toml`
**ESP-IDF:** v5.3.3
**Toolchain:** `esp` (fork Xtensa de Rust — ver `rust-toolchain.toml`)

## Targets soportados

| Chip | Target | Arquitectura | Notas |
|---|---|---|---|
| ESP32 (rev1) | `xtensa-esp32-espidf` | Xtensa LX6 | board de dev clásica |
| ESP32-S3 | `xtensa-esp32s3-espidf` | Xtensa LX7 | mayor RAM, USB-OTG, soporte PSRAM |
| ESP32-C3 | `riscv32imc-esp-espidf` | RISC-V | requiere toolchain `esp` con target RISC-V |
| ESP32-C6 | `riscv32imac-esp-espidf` | RISC-V | Wi-Fi 6 + Zigbee/Thread |

Para cambiar de chip editá dos líneas en `.cargo/config.toml`:

```toml
[build]
target = "xtensa-esp32s3-espidf"   # ← target del chip destino

[env]
MCU = "esp32s3"                     # ← nombre del MCU para ESP-IDF
```

## PSRAM en ESP32-S3

La ESP32-S3 puede incluir PSRAM externa (modelos N8R2, N16R8, etc.).
Habilitarla permite usar hasta 8 MB adicionales de heap sin modificar el código Rust.

Agregá en `sdkconfig.defaults` según el modelo:

```ini
# PSRAM básico (SPI, modelos N8R2)
CONFIG_SPIRAM=y

# PSRAM OPI/Octal (modelos N16R8 — la mayoría de DevKit S3)
CONFIG_SPIRAM=y
CONFIG_SPIRAM_MODE_OCT=y
CONFIG_SPIRAM_SPEED_80M=y
```

**Casos de uso típicos con PSRAM:**
- Buffers de audio / DSP donde la SRAM interna (512 KB) no alcanza.
- Framebuffers para displays (LVGL, e-paper).
- Colas grandes de FreeRTOS o estructuras `Vec` con datos históricos de sensores.
- Procesamiento de imágenes embebido (inferencia TensorFlow Lite Micro).

## Setup

```sh
rustup toolchain install stable
rustup run stable cargo install espup
espup install
```

```sh
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
```

Flasheo manual según target activo:

```sh
# ESP32 (rev1)
espflash flash target/xtensa-esp32-espidf/debug/obrero-1 && espflash monitor

# ESP32-S3
espflash flash target/xtensa-esp32s3-espidf/debug/obrero-1 && espflash monitor
```

## Entorno de testeo

[Web MIDI Tester](https://studiocode.dev/webmidi-tester/midi)