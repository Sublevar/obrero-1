# obrero-1

Secuenciador MIDI de pasos en Rust, con doble target: **ESP32** (MIDI DIN por UART) y **web** (WASM + Web MIDI API). Toda la lógica vive en un core portable; las plataformas solo aportan tiempo e I/O.

## Arquitectura

```
crates/obrero-core    Motor sans-io: parser MIDI, patrón, transporte, reloj 480 PPQN.
                      Sin threads ni I/O — la plataforma llama advance(now, lookahead)
                      y recibe eventos MIDI timestampeados. Testeable en host.
crates/obrero-wasm    Bindings wasm-bindgen del motor para el navegador.
web/                  UI TypeScript (vite): grilla de pasos, Web MIDI, scheduler
                      con lookahead de 100 ms y send timestampeado.
firmware/             Binario ESP32 (esp-idf-svc, toolchain xtensa propio).
                      Excluido del workspace raíz — se compila aparte.
```

- Reloj interno (master, emite 0xF8/Start/Stop) o externo (esclavo de MIDI clock entrante).
- Resolución interna 480 PPQN (ver "Resolución temporal"); el MIDI clock sigue a 24 PPQN y un paso de semicorchea = 120 ticks.
- `MIDI-DIN.md` documenta el hardware DIN (6N137, UART2 a 31250 bps).
- [PPQN](https://noiseengineering.us/blogs/loquelic-literitas-the-blog/getting-started-clocks/)
## Resolución temporal (PPQN)

PPQN (*pulses per quarter note*) es cuántos ticks internos caben en una negra: la grilla más fina en la que el motor puede ubicar una nota, medir un gate o repartir una subdivisión. El motor cuenta todo en ticks (`pattern::PPQN`, definido en `tuning.rs`); el tempo solo cambia cuánto dura cada tick.

Se fija en **480** porque es divisible por 24 (el MIDI clock sigue saliendo a 24 PPQN: un `0xF8` cada 20 ticks) y por 3, 4, 5, 8, 16…, así que tresillos, quintillos y micro-timing caen en ticks enteros. A 120 bpm un tick dura ~1,04 ms; a 300 bpm, ~208 µs.

Cuesta performance: a 300 bpm son 2400 ticks por segundo que el reloj genera y el motor procesa, y los rings (más abajo) crecen en proporción. El PPQN puede **reducirse** (p. ej. a 96 o 24) para ganar performance, sobre todo en el ESP32, a costa de granularidad y resolución del micro-timing. Debe seguir siendo múltiplo de 24; los demás topes se recalculan solos al cambiarlo.

## Ajustes (fine tuning)

Todas las variables de ajuste viven en `crates/obrero-core/src/tuning.rs`. Los tamaños de ring se derivan de las demás, así que cambiar una mantiene coherente al resto.

| Variable | Valor | Qué controla / qué pasa si se cambia |
|---|---|---|
| `PPQN` | 480 | Resolución interna. Más alto: más fino y más costoso. Debe ser múltiplo de `MIDI_CLOCK_PPQN`. |
| `MIDI_CLOCK_PPQN` | 24 | Resolución del `0xF8`. Fijada por el estándar MIDI. |
| `MIN_BPM` / `MAX_BPM` | 20 / 300 | Rango de tempo. `MAX_BPM` dimensiona los rings (peor caso). |
| `MAX_SUBS` | 32 | Suscripciones simultáneas a un `Clock`. Superado: `ClockError::Full`. |
| `PER_BAR_MAX` | 96 | Tope de `Subdivision::PerBar`. Dimensiona `NOTICE_CAPACITY`. |
| `EVERY_N_BARS_MAX` | 8 | Tope de `Subdivision::EveryNBars`. |
| `MAX_CATCHUP_US` | 100 000 | Atraso que el reloj recupera tick por tick. Más allá salta hacia adelante (`skipped_ticks`) en vez de emitir una ráfaga. |
| `MAX_LOOKAHEAD_US` | 100 000 | Lookahead por pasada que los rings cubren. `Engine::advance` parte lookaheads mayores en pasadas de este tamaño. La web usa 100 ms; el ESP32, 0. |
| `RING_MARGIN` | 4 | Holgura sumada al peor caso de cada ring, antes de redondear a potencia de 2. |
| `EXTERNAL_SMOOTHING` | 4 | Clock externo: el período entre `0xF8` se suaviza como `(anterior · (N − 1) + medido) / N`. Más alto filtra más jitter y sigue más lento los cambios de tempo. |
| `NOTICE_CAPACITY` | derivada (32) | Ring por defecto de una suscripción: avisos de `PerBar(PER_BAR_MAX)` en `MAX_CATCHUP_US + MAX_LOOKAHEAD_US` a `MAX_BPM`, redondeado a potencia de 2 (índice con máscara, sin división). 512 B por suscripción; se aloja al suscribir, no en el camino caliente. |
| `ENGINE_RING_CAPACITY` | derivada (512) | Ring de la suscripción del `Engine` (un aviso por tick base) en la misma ventana, potencia de 2: 8 KiB (16 B por aviso). Con estos topes no puede desbordar; `Engine::lost_ticks()` lo verifica y debe ser 0. |

Si un ring llega a llenarse, el aviso nuevo se descarta y se cuenta (`Subscription::take` devuelve los perdidos); nunca se pierde en silencio. Para un uso concreto se puede pedir otro tamaño con `Clock::subscribe_with_capacity` (se redondea a la siguiente potencia de 2).

La memoria la fija la ventana (`MAX_CATCHUP_US + MAX_LOOKAHEAD_US`), no el PPQN: duplicar la ventana duplica el ring del `Engine` (512 → 1024 → 2048 avisos, 8 → 16 → 32 KiB). En el ESP32-S3 conviene dejarla corta. El reloj externo (`0xF8`) también pasa por el `Clock`: el tempo se mide con el período entre pulsos y cada pulso reparte sus 20 ticks parejo, sin esperar al pulso siguiente.

## Workspace raíz (stable)

```sh
# Tests del motor (host)
cargo test -p obrero-core

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

cargo build              # primera build descarga ESP-IDF en .embuild/ — tarda
./dev.sh                 # build + flash + monitor por el conector UART del devkit

./dev.sh -p /dev/ttyUSB1 # elegir puerto serie a mano

./dev.sh -m esp32        # flashear el binario de la board ESP32 clásica

```

`dev.sh -m` solo elige qué directorio de target flashear — el chip compilado lo define `.cargo/config.toml`.

### USB-MIDI + debug serial (dos cables)

El firmware S3 levanta TinyUSB (componente `espressif/esp_tinyusb`, ver
`Cargo.toml` y `CONFIG_TINYUSB_MIDI_COUNT` en `sdkconfig.defaults`) sobre el
puerto **USB nativo** (GPIO19/20), que enumera como dispositivo USB-MIDI.
Los logs y el flasheo van por el conector **UART** (bridge USB-UART → UART0),
así que conviene tener ambos cables conectados: `dev.sh` flashea y monitorea
solo por `/dev/ttyUSB*` y deja el `/dev/ttyACM*` (USB nativo) libre para MIDI.

Al usar USB-OTG se pierde el USB-Serial-JTAG del puerto nativo (comparten PHY);
el debug queda en el conector UART. TinyUSB no compila para el ESP32 clásico
(no tiene USB-OTG): para ese chip hay que comentar el bloque
`extra_components` en `firmware/Cargo.toml`.

Flasheo manual según target activo:

```sh
# ESP32 (rev1)
espflash flash target/xtensa-esp32-espidf/debug/obrero-1 && espflash monitor

# ESP32-S3
espflash flash target/xtensa-esp32s3-espidf/debug/obrero-1 && espflash monitor
```

## Entorno de testeo

[Web MIDI Tester](https://studiocode.dev/webmidi-tester/midi)

## Licencia

[GPL-3.0-or-later](LICENSE) — © 2026 Pablo Labarta <pablitolabarta@gmail.com> & Santiago Fernandez <stfg.prof@gmail.com>.
