# Clock / Observer — Time, Bpm, Clock, Sequencer, Debug

**obrero-1, core (`crates/obrero-core`).** Fecha: 2026-10-02. Status: implementado en `crates/obrero-core/src/clock.rs`, con binding web en `crates/obrero-wasm/src/clock.rs`. `Engine` ya lo usa (se suscribe a un `Clock` propio, §5) y el PPQN interno es 480 (§3). Las variables de ajuste están en `crates/obrero-core/src/tuning.rs`, documentadas en el `README.md`.

Sesiones:
- `docs/sessions/2026-10-01-clock-observer-design.md`: diseño.
- `docs/sessions/2026-10-01-clock-implementacion.md`: implementación, y correcciones al diseño aprobadas durante ella (§11).
- `docs/sessions/2026-10-02-ppqn-480-engine-clock.md`: PPQN 480, `Engine` migrado a `Clock`, topes y rings (§3, §4.2, §5, §11).
- `docs/sessions/2026-10-02-clock-externo-y-rings.md`: reloj externo dentro del `Clock`, rings potencia de 2, `WasmClock` eliminado (§4.2, §5, §7, §11).

---

## 0. Resumen

Separa lo que hoy vive fusionado dentro de `Engine` (`engine.rs`) en piezas independientes:

- **`Time`**: una marca temporal monotónica, un singleton por programa corriendo.
- **`TimeSource`**: la fachada para leer el reloj real. Cada plataforma tiene su implementación.
- **`Bpm`**: tempo en centésimas, uno por `Clock`.
- **`Clock`**: el Observer. Lee `Time`, genera ticks base con su instante exacto según su `Bpm`, y lleva una tabla de suscripciones con subdivisión configurable.
- **`Sequencer`**: contrato (no implementación) para quien consume los avisos de una suscripción y produce MIDI.
- **`Debug`**: un consumidor más, que traduce avisos a bar/beat y los emite a un sink inyectable.

Esto habilita lo que pedía el enunciado original: varios secuenciadores colgados del mismo `Clock`, con subdivisiones distintas y un único bpm. Deja la puerta abierta, sin construirla todavía, a varios `Clock`.

## 1. Decisiones de arquitectura (el "por qué" antes del "qué")

1. **Modelo pull, no push.** El `Clock` nunca ejecuta código ajeno dentro de su propio tick. En cada tick base, lo único que hace por cada suscripción debida es encolar un aviso `Notice { tick, at_us }`. El consumidor lo retira cuando *a él* lo hacen avanzar.
   Un Observer push tiene problemas conocidos para este caso: bloquea al Subject si un observer tarda, puede causar reentrancia o deadlock, y suele pedir callbacks `Box<dyn FnMut>` (heap en el camino caliente de un `no_std` que corre horas en ESP32).
   El aviso lleva el **instante exacto** del tick, no solo un conteo. La web programa los eventos 100 ms adelante, así que sin ese instante el jitter sería el del pump (~25 ms).
2. **`Time` es el singleton real, no `Clock`.** Una sola marca temporal monotónica en µs por programa, sin noción musical. La crea una vez el punto de entrada de cada plataforma y la alimenta con lo que lee su `TimeSource`. Es singleton por construcción (un solo punto de creación, pasada por referencia), no un `static`.
3. **Fachada `TimeSource` para leer el reloj; aritmética única en el core.** Cada plataforma implementa `TimeSource` de la forma más eficiente que tenga:
   - web: `performance.now()` llega como `now_ms` desde JS (no hay `WebTime`);
   - `EspTime`: `esp_timer_get_time()`, pendiente;
   - en los tests, un valor controlado.

   El cálculo de cuándo cae cada tick no tiene variantes por plataforma: es uno solo, entero (§3), para que web y hardware se comporten igual.
4. **`Bpm` es por instancia de `Clock`, no global.** Cada `Clock` tiene su propio `Bpm`, y todos se anclan a la misma `Time`. `Clock::set_bpm` es el "cambiar bpm global" del enunciado: global al `Clock`, no al programa.
5. **Subdivisión flexible por default; modo precisión opt-in.** Ver §3.
6. **Auto-desuscripción vía `Drop`, con la suscripción co-poseyendo el clock por `Rc`.** Rust ya da el "destructor unregisters" con `Drop`, sin macro.
   La suscripción guarda un `Rc<Clock>`, no un `&'clock Clock`. Un préstamo con lifetime no puede cruzar a JS (wasm-bindgen no admite lifetimes) ni convivir con el `Clock` dentro de un mismo struct. Con `Rc` no hay lifetime, y en JS el `.free()` de la suscripción dispara el `Drop`.
   Límite: `Rc` es de un solo hilo. En la web siempre lo es; en ESP32, una task por clock. Si hiciera falta compartirlo entre tasks, se pasa a `Arc`.
7. **Sin `async`/`await`.** "Async" en el enunciado se interpreta como "no bloqueante": el modelo pull ya lo es, sin executor.

## 2. A) `Time` y `TimeSource`

```rust
pub trait TimeSource {
    fn now_us(&self) -> u64;
}

pub struct Time { now_us: u64 }

impl Time {
    pub const fn new() -> Self;
    /// Satura hacia adelante: un valor atrasado no hace retroceder el tiempo.
    pub fn advance(&mut self, now_us: u64);
    pub fn now_us(&self) -> u64;
}
```

Uso en cada pump de la plataforma: `time.advance(source.now_us()); clock.advance(&time, lookahead_us);`.

## 3. B) `Bpm` y `Subdivision`

```rust
/// Centésimas de bpm (12000 = 120.00), clamp a [MIN_BPM, MAX_BPM] de engine.rs.
pub struct Bpm(u32);

impl Bpm {
    pub const fn from_centi(centi: u32) -> Self;
    pub fn from_f32(bpm: f32) -> Self; // NaN → mínimo
}

pub enum Subdivision {
    /// n avisos por bar. 1..=PER_BAR_MAX (96).
    PerBar(u8),
    /// 1 aviso cada n bars, al inicio del bar. 1..=EVERY_N_BARS_MAX (8).
    EveryNBars(u8),
    /// Un aviso por tick base. Es el consumo del `Engine`.
    EveryTick,
}
```

**Tiempo exacto, entero.** El instante del tick `n` es `ancla_us + (n − ancla_tick) · 6·10⁹ / (centi_bpm · PPQN)`, calculado en `u128`.
- No acumula error: cada tick se calcula desde el ancla, no sumando un período redondeado.
- Da el mismo resultado bit a bit en xtensa (ESP32-S3 no tiene FPU de 64 bits; `f64` se emularía por software) y en wasm.
- Cambiar el bpm crea un ancla nueva en el próximo tick todavía no generado: los ticks ya emitidos dentro del lookahead conservan su instante.

**Dos variantes** porque "subdivisión" cubre dos direcciones del mismo eje: más rápido que un bar (`PerBar`) y más lento (`EveryNBars`).

**Alineación al compás.** `hits(tick)` es un predicado puro sobre el tick base: tick 0 = inicio del bar 0, y un bar = `PPQN × 4 = 1920` ticks (4/4, con `PPQN = 480`). Dos suscripciones con la misma subdivisión avisan en los mismos ticks, sin importar cuándo se suscribieron.

**Resolución de `PerBar(n)`:**
- **Flexible (default):** reparto `E(n, TICKS_PER_BAR)` con `euclidean_hit` (`pattern.rs`), la misma fórmula de los ritmos euclidianos. Es entera y determinística. Los intervalos entre avisos pueden variar en ±1 tick base: con `PerBar(7)`, 274 o 275 ticks.
- **Precisión (opt-in, `Clock::set_precision_mode(bool)`):** ajusta `n` al divisor exacto de `TICKS_PER_BAR` más cercano dentro de `1..=PER_BAR_MAX`; ante empate, al menor. Con 480 PPQN son `{1,2,3,4,5,6,8,10,12,15,16,20,24,30,32,40,48,60,64,80,96}`, calculados en `snap_precise` (no hay lista fija). Nunca rechaza. El valor efectivo se informa (`Subscription::subdivision`, `set_subdivision`). Cambiar el modo re-resuelve todas las suscripciones vivas desde lo que pidió cada una.

`EveryNBars(n)` siempre es exacto. Una subdivisión fuera de rango da `ClockError::OutOfRange`.

**Límites.** El tope de `EveryNBars` (8) es de producto. El de `PerBar` pasó de 24 a 96 al subir el PPQN (§11); todos los topes son variables de `tuning.rs`.

**PPQN = 480** (antes 24). Múltiplo de 24: el MIDI clock sigue a 24 PPQN y `Engine` emite un `0xF8` cada `TICKS_PER_MIDI_CLOCK = 20` ticks. `gate_ticks` y `ticks_per_step` pasan a `u16`, y `TimeMark.tick_in_beat` a `u16`.

## 4. C) `Clock` (el Observer) y `Subscription`

### 4.1 Mutabilidad interior

Toda la API de `Clock` usa `&self`: lo comparten la plataforma y cada `Subscription` (vía `Rc`). El estado mutable vive en `Cell`/`RefCell` (en `core`, sin `unsafe`).

`advance` es lo único que toma el `RefCell` de los slots, y mientras lo tiene no ejecuta código ajeno. Por eso el borrow del `Drop` de una `Subscription` nunca falla por reentrancia. Igual se usa `try_borrow_mut` + `debug_assert!`: un bug salta en los tests sin provocar un panic en producción.

### 4.2 Forma

```rust
pub const MAX_SUBS: usize = 32;
pub const NOTICE_CAPACITY: usize = 32;       // por suscripción (derivada, potencia de 2)
pub const ENGINE_RING_CAPACITY: usize = 512; // ring del Engine (derivada, potencia de 2)
pub const MAX_CATCHUP_US: u64 = 100_000;
pub const MAX_LOOKAHEAD_US: u64 = 100_000;
pub const EXTERNAL_SMOOTHING: u64 = 4;

pub struct Notice { pub tick: u64, pub at_us: u64 }

impl Clock {
    pub fn new(bpm: Bpm) -> Rc<Clock>;
    /// Genera los ticks con instante <= now + lookahead y encola avisos.
    /// La primera llamada (o la primera tras `reset`) ancla el tick 0 en `now`.
    pub fn advance(&self, time: &Time, lookahead_us: u64);
    /// Igual, con horizonte absoluto: permite partir un lookahead largo.
    pub fn advance_to(&self, now_us: u64, horizon_us: u64);
    /// Instante del próximo tick a generar; None hasta el primer advance.
    pub fn next_tick_at_us(&self) -> Option<u64>;
    pub fn set_bpm(&self, bpm: Bpm);
    pub fn bpm(&self) -> Bpm;
    pub fn set_precision_mode(&self, on: bool);
    /// Tick 0 en el próximo advance; descarta avisos pendientes.
    pub fn reset(&self);
    pub fn tick(&self) -> u64;
    pub fn skipped_ticks(&self) -> u64;
    pub fn subscribe(self: &Rc<Self>, s: Subdivision) -> Result<Subscription, ClockError>;
    /// Ring de `capacity` avisos en vez de `NOTICE_CAPACITY`, redondeado a
    /// la siguiente potencia de 2 (0 -> 1).
    pub fn subscribe_with_capacity(self: &Rc<Self>, s: Subdivision, capacity: usize)
        -> Result<Subscription, ClockError>;
}

impl Subscription {
    /// Mueve los avisos a `out`; devuelve los perdidos por ring lleno.
    pub fn take(&self, out: &mut Vec<Notice>) -> u32;
    pub fn subdivision(&self) -> Subdivision; // efectiva
    pub fn set_subdivision(&self, s: Subdivision) -> Result<Subdivision, ClockError>;
    pub fn unsubscribe(self);
}
// Drop: libera el slot si su `generation` coincide.
```

**Garantías de fiabilidad:**
- **Ring por suscripción, reservado al suscribir y sin asignaciones en el camino caliente** (`push`/`take` no asignan). Si se llena, el aviso nuevo se descarta y se cuenta; `take` devuelve cuántos se perdieron. Nunca se pierde en silencio.
- **Los rings se dimensionan para el peor caso de una pasada de `advance`**: atraso máximo recuperable (`MAX_CATCHUP_US`) + lookahead máximo (`MAX_LOOKAHEAD_US`), a `MAX_BPM`. `NOTICE_CAPACITY` cubre `PerBar(PER_BAR_MAX)`; `ENGINE_RING_CAPACITY` cubre un aviso por tick base (2400 ticks/s a 300 bpm). Se derivan en `tuning.rs` y se redondean a **potencia de 2**, para indexar con máscara en vez de división (relevante en el ESP32): cambiar PPQN, tempo o ventanas los recalcula. La memoria la fija la ventana, no el PPQN: con 100 + 100 ms el ring del `Engine` es de 512 avisos (8 KiB a 16 B por aviso). Quien use `Clock` directo debe respetar `MAX_LOOKAHEAD_US` por pasada; `Engine::advance` parte lookaheads mayores.
- **Tabla llena:** `ClockError::Full`. No crece dinámicamente.
- **Atraso grande** (pestaña dormida, laptop suspendida): si `now` supera el próximo tick por más de `MAX_CATCHUP_US` (100 ms), el clock salta hacia adelante sobre la misma grilla, en vez de emitir una ráfaga de ticks vencidos, y lo suma a `skipped_ticks()`.
- **Slots con `generation`:** un handle viejo nunca libera ni lee un slot reusado.

### 4.3 Operaciones del enunciado, mapeadas

| Pedido original | Dónde queda |
|---|---|
| Crear un clock | `Clock::new(bpm)` |
| Suscribirte a un clock, con subdivisión | `Clock::subscribe(subdivision)` |
| Cambiar subdivisión de una suscripción | `Subscription::set_subdivision(new)` |
| Desuscribirte | `Subscription::unsubscribe()`, o automático al Drop (`.free()` en JS) |
| Notificar a los grupos de cada subdivisión | Dentro de `Clock::advance`: los slots con la misma subdivisión reciben aviso en el mismo tick |
| Cambiar bpm global | `Clock::set_bpm(bpm)` |
| Obtener mensajes/ticks pendientes | `Subscription::take(out)`, con instante exacto |

## 5. D) `Sequencer` y `Engine`

```rust
pub trait Sequencer {
    fn on_notices(&mut self, notices: &[Notice], out: &mut Vec<TimedMidi>);
}
```

Uso en la plataforma, por cada secuenciador suscripto: `sub.take(&mut buf); seq.on_notices(&buf, out);`.

Cada aviso trae su `at_us`, así que el secuenciador estampa sus `TimedMidi` con el instante exacto del tick.

**`Engine` implementa `Sequencer` y es su propio consumidor.** `Engine::new` crea un `Clock` (120 bpm) y se suscribe con `Subdivision::EveryTick` y un ring de `ENGINE_RING_CAPACITY`. En reloj interno, `Engine::advance(now, lookahead, out)`:
1. `play()` hace `Clock::reset()`; el primer `advance` ancla el tick 0 en `now`;
2. avanza `Time`, y `Clock::advance_to(now, horizonte)` en pasadas de a lo sumo `MAX_LOOKAHEAD_US`;
3. tras cada pasada retira los avisos (`take`) y llama `on_notices`, que dispara note-on/off y emite `0xF8` en los ticks múltiplos de `TICKS_PER_MIDI_CLOCK`;
4. con transporte detenido no avanza el clock. `Stopping` y el doble Stop conservan su semántica; el corte final resetea el `Clock`.

`set_tempo`/`bpm` delegan en `Clock::set_bpm`/`bpm` (centésimas, sin `f64`). `Engine::clock()` entrega el `Rc<Clock>` para colgarle otras suscripciones (p. ej. `DebugTap`); `Engine::lost_ticks()` debe ser siempre 0. 
**Reloj externo, en el mismo `Clock`.** `Engine::set_clock_source(External)` pone el `Clock` en `ClockSource::External` (`Clock::set_source`, que también hace `reset`). Cada `0xF8` entrante llama a `Clock::external_pulse(now_us)`, que:
1. mide el período entre pulsos y lo suaviza con `EXTERNAL_SMOOTHING`, acotado a `MIN_BPM..=MAX_BPM`; un silencio de más de dos pulsos lentos se trata como corte y no como tempo;
2. ajusta el `Bpm` del clock al medido (`Clock::bpm()`);
3. genera los `TICKS_PER_MIDI_CLOCK` ticks de ese pulso, repartidos parejo a lo largo del período medido y desde la llegada del pulso (predicción: latencia 0). Las estampas no decrecen: si la predicción de un pulso se pasa, el siguiente arranca en el último instante emitido.

El `Engine` consume esos avisos por la misma suscripción `EveryTick` que en reloj interno. Con el transporte detenido los pulsos igual miden el tempo, pero sus avisos se descartan; el contador de ticks del motor sobrevive a un Stop/Continue. En esclavo no se re-emite `0xF8`. En `External`, `advance` no genera ticks.


## 6. E) `Debug`

```rust
pub struct TimeMark { pub bar: u32, pub beat: u8, pub tick_in_beat: u8, pub at_us: u64 }

/// El core no llama `println!`: cada plataforma inyecta su sink.
pub trait DebugSink { fn emit(&mut self, mark: TimeMark); }

/// Consumidor con su propia Subscription; traduce avisos a TimeMark (4/4).
pub struct DebugTap { /* … */ }
impl DebugTap {
    pub fn new(clock: &Rc<Clock>, s: Subdivision) -> Result<Self, ClockError>;
    pub fn poll(&mut self, sink: &mut impl DebugSink) -> u32; // perdidos
}
```

Sinks:
- tests de host: un `Vec<TimeMark>`;
- web: `console.log` (`WasmDebugTap`; con `?clockdebug` en la URL lo activa `web/src/main.ts`);
- ESP32: `log::info!` sobre UART0, pendiente.

`TimeMark.tick_in_beat` es `u16` (0..PPQN).

## 7. Plataforma web (`crates/obrero-wasm`)

- Un solo `Clock` en la web: el de `WasmEngine`. Los timestamps llegan como `now_ms` de `performance.now()` o de Web MIDI; no hay `WebTime` ni un `Clock` aparte (se eliminaron `WasmClock`, `WasmSubscription` y `WebTime`). El trait `TimeSource` queda en el core para `EspTime`.
- `WasmSubscription`: `take()` devuelve `[tick, at_ms] * N` como `Float64Array`, `lost()` informa los perdidos, y `.free()` desuscribe.
- `WasmEngine` usa su `Clock` propio: `bpm()`, `next_tick_ms()` (NaN si no corre) y `debug_tap(per_bar)` cuelgan de él. `WasmDebugTap` vive en `crates/obrero-wasm/src/debug.rs`.
- **Pérdidas.** `Engine::loss_report()` (`LossReport`) acumula tres contadores: `lost_ticks` (ring del motor lleno, debe ser 0), `skipped_ticks` (ticks que el reloj saltó tras un atraso mayor a `MAX_CATCHUP_US`) y `skipped_steps` (pasos del patrón dentro de esos ticks: no suenan). En reloj interno el motor toma su `tick` del aviso, así que tras un salto los pasos siguientes caen donde corresponde en la grilla. `WasmEngine::loss_report()` los entrega como `[lost_ticks, skipped_ticks, skipped_steps]`.
- **`LossMonitor`** (`web/src/lossmonitor.ts`) lee el reporte en cada pump del scheduler (más los avisos perdidos del `DebugTap`) y avisa por `console.warn` y, solo en `vite dev`, con un POST a `/__obrero/loss`, que `web/vite.config.ts` imprime en la terminal de vite. Agrupa los avisos (uno cada 250 ms como máximo). Se duermen con el botón del panel de debug, `window.obreroLoss.muted = true` o `?losslog=off`; dormidos, los contadores siguen y el indicador del panel los muestra.
- `?clockdebug` en la URL activa en `web/src/main.ts` un panel fijo con bpm, `now`, instante del próximo tick y su diferencia, gap del pump e indicador de pérdidas (en rojo si hubo alguna), más el log por consola del `DebugTap` y de `next_tick_ms`. Para comparar precisión: las diferencias entre `next_tick_ms` sucesivos deben ser un período de tick exacto (a 120 bpm, 1041,667 µs).

## 8. Preguntas que quedaban abiertas en el borrador original, resueltas

- **"Time es un singleton… los clocks se derivan de leer el ___"**: los `Clock` derivan de leer `Time`, no la poseen (§1.2).
- **"Bpm puede haber múltiples" vs. "1 único bpm por instancia de observer"**: no se contradicen si `Bpm` vive en el `Clock` (§1.4).
- **"Requisitos: Async"**: no bloqueante vía modelo pull (§1.7).
- **"Algún macro" para auto-desuscripción**: `Drop` (§1.6).
- **Time signature**: fijo en 4/4 en esta versión (96 ticks por bar, §3 y §6).

## 9. Fuera de alcance (explícitamente, para no perderlo)

- `EspTime` y uso del core en el firmware (el firmware todavía no linkea `obrero-core`, a propósito: MVP-0).
- Múltiples `Clock` simultáneos: el diseño no lo impide, pero no se construye.
- Reemplazo de `midi.rs` por una librería externa.
- Time signatures distintas de 4/4.

## 10. Mejoras propuestas, no aprobadas

Se listan para decidir; no se da por hecha su utilidad.

- **Start/stop de transporte en `Clock`.** Hoy solo hay `reset`; el transporte sigue en `Engine`.

## 11. Cambios respecto del diseño original (aprobados en la implementación)

| Diseño original | Implementado | Por qué |
|---|---|---|
| `Subscription<'clock>` con `&'clock Clock` | `Subscription` con `Rc<Clock>`; `Clock::new` devuelve `Rc<Clock>` | Exponer a JS y poder guardar clock y suscripciones juntos (§1.6) |
| `pending: u32` + `take_pending()` | Ring de `Notice { tick, at_us }` + `take(out) -> perdidos` | Instante exacto por tick para el lookahead de la web (§1.1) |
| `Bpm(f32)` | `Bpm(u32)` en centésimas, aritmética `u128` | Exacto y sin deriva; sin FPU de 64 bits en ESP32-S3 (§3) |
| `Time` alimentado directo por la plataforma | `Time` + fachada `TimeSource` por plataforma | Lectura eficiente por target con un solo contrato (§1.3) |
| `Sequencer::on_ticks(ticks: u32, …)` | `Sequencer::on_notices(&[Notice], …)` | El secuenciador necesita el instante de cada aviso (§5) |
| — | `Clock::reset()`, `MAX_CATCHUP_US` + `skipped_ticks()` | Alinear el compás al dar Play; no emitir ráfagas tras un atraso (§4.2) |
| `interval_ticks` resuelto por slot | Predicado `Subdivision::hits(tick)` | El reparto euclidiano no es un intervalo fijo; el predicado es O(1) y está alineado al compás (§3) |
| PPQN 24, `PerBar` hasta 24, ring fijo de 16 inline, `MAX_CATCHUP_US` 1 s | PPQN 480, `PerBar` hasta 96, ring reservado al suscribir, dimensionado por peor caso y redondeado a potencia de 2, catch-up y lookahead de 100 ms, todo en `tuning.rs` | Mayor resolución; `Engine` necesita un aviso por tick (2400/s a 300 bpm) sin pérdida (2026-10-02, pedido del usuario) |
| `Engine` con reloj `f64` propio | `Engine` suscripto a su `Clock` con `EveryTick` + `Sequencer` | Una sola fuente de tiempo, entera y exacta; el reloj se prueba en la web real (2026-10-02) |
| `Engine` con camino propio para el 0xF8 externo (ráfaga de 20 ticks) | `Clock::external_pulse` + `ClockSource` en el `Clock`; `Engine` consume por la misma suscripción | Un solo reloj, con tempo medido e interpolación (2026-10-02, pedido del usuario) |
| `WasmClock`/`WasmSubscription`/`WebTime` | Eliminados; `WasmDebugTap` en `debug.rs` | Un solo `Clock` en la web (2026-10-02) |
| — | `Subdivision::EveryTick`, `Clock::advance_to`, `next_tick_at_us`, `subscribe_with_capacity` | Soporte del consumo del `Engine` y del debug |

## 12. Testing

`crates/obrero-core/tests/clock.rs`, en host, sin hardware:
- deriva nula en 5 minutos simulados con las ventanas de la web, y en 1 hora;
- instantes exactos;
- ventanas solapadas sin duplicados;
- cambio de bpm sin saltos;
- `PerBar(7)` y modo precisión;
- `EveryNBars`;
- 32 suscripciones en el mismo `advance`;
- `ClockFull` y liberación por Drop;
- desborde contado (ring de capacidad elegida);
- `EveryTick`: un aviso por tick, exactos;
- peor caso de una pasada sin pérdida con los rings por defecto;
- `Engine`: 0xF8 a 24 PPQN, deriva a 300 bpm, atraso largo sin perder ticks, suscripciones extra al clock del motor;
- clock externo: ticks repartidos parejo dentro de cada pulso, tempo medido y rampa, jitter filtrado sin deriva (5 min), silencio como corte, Continue que retoma el patrón;
- rings potencia de 2 y redondeo de la capacidad pedida;
- salto por atraso;
- `reset`;
- `DebugTap`.

`no_std` verificado compilando para `xtensa-esp32s3-none-elf` (job de CI `core-no-std-xtensa`).
