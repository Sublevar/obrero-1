MODEL="esp32s3" # "esp32" #esp32s3 

while [[ $# -gt 0 ]]; do
  case "$1" in
    -m|--modelo)
      MODEL="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

cargo build

espflash flash target/xtensa-${MODEL}-espidf/debug/obrero-1

espflash monitor