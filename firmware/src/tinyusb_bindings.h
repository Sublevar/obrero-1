// Header de bindings para esp-idf-sys (ver extra_components en Cargo.toml).
// tinyusb.h arrastra tusb.h, que con CONFIG_TINYUSB_MIDI_COUNT>=1 expone la
// API MIDI de dispositivo (tud_midi_n_stream_write / read, descriptores TUD_*).

// El clang de bindgen no recibe el include-path "pelado" del kernel FreeRTOS
// que agrega el CMake del componente; con este prefijo el osal de TinyUSB
// incluye "freertos/FreeRTOS.h", que sí resuelve.
#define CFG_TUSB_OS_INC_PATH freertos/

#include "tinyusb.h"
