#!/usr/bin/env bash
set -euo pipefail

# Setup recomendado para ESP32-S3 + USB-MIDI (dos cables):
#   conector "UART" del devkit -> flash + monitor serial (este script)
#   conector "USB" nativo      -> dispositivo USB-MIDI via TinyUSB, queda libre
# El bridge UART aparece como /dev/ttyUSB*; el USB nativo como /dev/ttyACM*.
# Forzamos el puerto para que espflash no flashee/monitoree el ACM por error.

MODEL="esp32s3" # debe coincidir con el target de .cargo/config.toml (esp32 | esp32s3)
PORT=""
FEATURES=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -m|--modelo)
      MODEL="$2"
      shift 2
      ;;
    -p|--puerto)
      PORT="$2"
      shift 2
      ;;
    -raw)
      FEATURES="logging-encoders-raw"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

if [[ -z "$PORT" ]]; then
  PORT=$(ls /dev/ttyUSB* 2>/dev/null | head -n1 || true)
fi

if [[ -z "$PORT" ]]; then
  echo "No encontré /dev/ttyUSB* (conector UART del devkit)." >&2
  echo "Conectá el puerto UART o indicá uno con -p /dev/ttyXXXX." >&2
  exit 1
fi

if [[ -n "$FEATURES" ]]; then
  echo "[dev.sh] compilando con features: $FEATURES"
  cargo build --features "$FEATURES"
else
  cargo build
fi

espflash flash --port "$PORT" "target/xtensa-${MODEL}-espidf/debug/obrero-1"

espflash monitor --port "$PORT"
