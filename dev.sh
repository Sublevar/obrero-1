MODEL="esp32s3" # "esp32" #esp32s3 
FEATURES=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -m|--modelo)
      MODEL="$2"
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

if [[ -n "$FEATURES" ]]; then
  echo "[dev.sh] compilando con features: $FEATURES"
  cargo build --features "$FEATURES"
else
  cargo build
fi

espflash flash target/xtensa-${MODEL}-espidf/debug/obrero-1

espflash monitor