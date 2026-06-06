# obrero-1

Secuenciador MIDI de pasos en Rust, con doble target: **ESP32** (MIDI DIN por UART) y **web** (WASM + Web MIDI API). Toda la lógica vive en un core portable; las plataformas solo aportan tiempo e I/O.

## Arquitectura

```
crates/obrero-core    Motor sans-io: parser MIDI, patrón, transporte, reloj 24 PPQN.
                      Sin threads ni I/O — la plataforma llama advance(now, lookahead)
                      y recibe eventos MIDI timestampeados. Testeable en host.
crates/obrero-wasm    Bindings wasm-bindgen del motor para el navegador.
web/                  UI TypeScript (vite): grilla de pasos, Web MIDI, scheduler
                      con lookahead de 100 ms y send timestampeado.
firmware/             Binario ESP32 (esp-idf-svc, toolchain xtensa propio).
                      Excluido del workspace raíz — se compila aparte.
```

- Reloj interno (master, emite 0xF8/Start/Stop) o externo (esclavo de MIDI clock entrante).
- Resolución interna 24 PPQN = MIDI clock; un paso de semicorchea = 6 ticks.
- `MIDI-DIN.md` documenta el hardware DIN (6N137, UART2 a 31250 bps).

## Workspace raíz (stable)

```sh
# Tests del motor (host)
cargo test -p obrero-core

<<<<<<< HEAD
# Build WASM
cargo build -p obrero-wasm --target wasm32-unknown-unknown
```

## Web

Requiere `wasm-pack` (`cargo install wasm-pack`) y Node.

```sh
cd web
npm install
npm run wasm   # compila crates/obrero-wasm → web/src/pkg
npm run dev    # http://localhost:5173 (Web MIDI requiere Chrome/Edge)
```

Elegí una salida MIDI (sintetizador, DAW o loopback virtual) y dale Play. Para modo esclavo, seleccioná una entrada MIDI y reloj "Externo".

## Firmware ESP32

Tiene su propio toolchain (`esp`, fork Xtensa — ver `firmware/rust-toolchain.toml`) y ESP-IDF v5.3.3 via embuild.

```sh
# Setup (una vez)
cargo install espup && espup install
source ~/export-esp.sh   # agregar al perfil de shell
cargo install espflash

cd firmware
cargo build      # primera build descarga ESP-IDF en .embuild/ — tarda
./dev.sh         # build + flash + monitor serial
=======
# Build + flash + monitor serial (todo junto)
./dev.sh
```

Flasheo manual según target activo:

```sh
# ESP32 (rev1)
espflash flash target/xtensa-esp32-espidf/debug/obrero-1 && espflash monitor

# ESP32-S3
espflash flash target/xtensa-esp32s3-espidf/debug/obrero-1 && espflash monitor
>>>>>>> 56ef925 (entorno esp32s3)
```

## Entorno de testeo

[Web MIDI Tester](https://studiocode.dev/webmidi-tester/midi)

## Licencia

[GPL-3.0-or-later](LICENSE) — © 2026 Pablo Labarta <pablitolabarta@gmail.com> & Santiago Fernandez <stfg.prof@gmail.com>.
