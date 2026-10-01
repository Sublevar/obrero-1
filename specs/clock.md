# Clock / Observer — Time, Bpm, Clock, Sequencer, Debug

**obrero-1, core (`crates/obrero-core`).** Status: diseño cerrado, implementación pendiente (próxima sesión). Reemplaza el borrador anterior de este mismo archivo — ese borrador quedaba con frases cortadas ("los clocks se derivan de leer el ___") y tensiones sin resolver (Time singleton vs. Bpm múltiple); esta versión las resuelve explícitamente en la sección 7.

Sesión que originó este documento: `docs/sessions/2026-10-01-clock-observer-design.md` (qué se leyó, qué se decidió, qué queda abierto).

---

## 0. Resumen

Separa lo que hoy vive fusionado dentro de `Engine` (`engine.rs`) en piezas independientes:

- **`Time`**: una marca temporal monotónica, un singleton por programa corriendo.
- **`Bpm`**: tempo + conversión tiempo real ⇄ ticks musicales, uno por `Clock`.
- **`Clock`**: el Observer — lee `Time`, avanza su propio contador de ticks según su `Bpm`, y lleva una tabla de suscripciones con subdivisión configurable.
- **`Sequencer`**: contrato (no implementación) para quien consume ticks de una suscripción y produce MIDI.
- **`Debug`**: un consumidor más, que traduce ticks a bar/beat y los emite a un sink inyectable.

Esto habilita lo que pedía el enunciado original: varios secuenciadores colgados del mismo `Clock` con subdivisiones distintas pero un único bpm, y deja la puerta abierta (sin construirla todavía) a varios `Clock` en el futuro.

## 1. Decisiones de arquitectura (el "por qué" antes del "qué")

1. **Modelo pull, no push.** El `Clock` nunca ejecuta código ajeno dentro de su propio tick. En cada tick base, lo único que hace por cada suscripción debida es incrementar un contador `pending`. El consumidor (`Sequencer`, `Debug`) lee ese contador cuando *a él* lo hacen avanzar — no al revés. Un Observer clásico (push: el Subject llama directamente al método del Observer) tiene problemas conocidos para este caso: bloquea al Subject si un observer tarda, puede causar reentrancia/deadlock, y normalmente pide guardar callbacks como `Box<dyn FnMut>` — heap, inaceptable en el camino caliente de un `no_std` que corre en ESP32 durante horas. El modelo pull evita los tres problemas y es, de hecho, el que ya insinuaba el pseudocódigo original de este archivo (`ClockSubscription::get_pending()` + `Seq::advance` que lo consulta).
2. **`Time` es el singleton real, no `Clock`.** Una sola marca temporal monotónica en microsegundos por programa corriendo, sin ninguna noción musical (sin bpm, sin subdivisión, sin bar). La crea una vez el punto de entrada de cada plataforma (la web o el firmware) y la alimenta empujando el valor que lee del reloj real (`performance.now()`, `esp_timer_get_time()`). Es singleton *por construcción* — un solo punto de creación, pasada por referencia hacia abajo — no un `static` de Rust (eso complicaría testear en host y pediría `unsafe` o un lock).
3. **`Bpm` es por instancia de `Clock`, no global.** Esto resuelve la tensión del borrador original ("Time es singleton" vs. "puede haber múltiples Bpm"): cada `Clock` (hoy uno; a futuro varios) tiene su propio `Bpm`, todos anclados a la misma `Time`. `Clock::set_bpm` es la operación de "cambiar bpm global" del enunciado — global al `Clock`, no a todo el programa.
4. **Subdivisión flexible por default; modo precisión opt-in.** Se explica en la sección 3.
5. **Auto-desuscripción vía `Drop`, no una macro.** El enunciado pedía "algún macro" para que al eliminarse el objeto suscriptor se desuscriba solo. Rust ya da esto gratis con el trait `Drop` — es exactamente el mecanismo que la literatura de Observer llama "destructor unregisters", la solución estándar al *lapsed listener problem*. No hace falta inventar nada.
6. **Sin `async`/`await`.** El borrador original pedía "Async" como requisito. Se interpreta como "no bloqueante", no como `async fn`: un executor es una dependencia que `no_std`/ESP32 no necesita acá, y el modelo pull ya es no bloqueante por construcción.

## 2. A) `Time`

```rust
pub struct Time {
    now_us: u64,
}

impl Time {
    pub const fn new() -> Self {
        Self { now_us: 0 }
    }

    /// La plataforma empuja el valor que lee de su reloj real. Satura hacia
    /// adelante: un timestamp que llega retrasado (jitter de la fuente) no
    /// hace retroceder el tiempo global.
    pub fn advance(&mut self, now_us: u64) {
        self.now_us = now_us.max(self.now_us);
    }

    pub fn now_us(&self) -> u64 {
        self.now_us
    }
}
```

Dueño: el punto de entrada de cada plataforma (uno solo — `main.rs` en firmware, el bootstrap de `web/src/main.ts` vía `WasmEngine` en la web). Se pasa por `&Time` a cuantos `Clock` la necesiten leer. `Time` no sabe que existe `Clock`: es deliberadamente la pieza más chica y más tonta de todo el diseño.

## 3. B) `Bpm` y `Subdivision`

```rust
pub struct Bpm(f32); // clamp [MIN_BPM, MAX_BPM] — reusa las constantes de engine.rs

pub enum Subdivision {
    /// n avisos repartidos lo más parejo posible dentro de un bar. 1..=24.
    PerBar(u8),
    /// 1 aviso cada n bars. 1..=8.
    EveryNBars(u8),
}
```

Dos variantes porque "subdivisión" cubre dos direcciones distintas del mismo eje: más rápido que un bar (`PerBar`, el ejemplo del enunciado es 1/24) y más lento que un bar (`EveryNBars`, el ejemplo es 8). No es la misma recta numérica — por eso no es una sola fracción.

**Límites:** `PerBar` tope en 24 porque coincide con el PPQN que ya usa `engine.rs` (24 ticks por negra): `PerBar(24)` es la resolución más fina que el motor puede dar hoy sin cambiar su reloj base. `EveryNBars` tope en 8 por el ejemplo del enunciado; no hay una razón técnica para no permitir más — es un límite de producto, ajustable.

**Resolución de `PerBar(n)` a ticks base:** asumiendo 4/4, un bar = `PPQN × 4 = 96` ticks base. Si `n` divide 96 exacto, los avisos quedan equiespaciados sin ambigüedad. Si no (p. ej. `PerBar(7)`, 96/7 ≈ 13.71), hay dos modos:

- **Flexible (default):** se reusa `euclidean_hits(k, n)` — ya en `pattern.rs`, la misma distribución tipo Bjorklund que usan los ritmos euclidianos — para decidir en qué tick base cae cada uno de los `n` avisos dentro de los 96 ticks del bar. Es entera (sin `f32`/`f64`, apta para `no_std`), determinística y sin deriva acumulada entre bares; el costo es que los intervalos entre avisos consecutivos pueden variar en ±1 tick base (≈0.8 ms a 120 bpm) en vez de ser todos idénticos.
- **Precisión (opt-in, `Clock::set_precision_mode(bool)`):** en vez de aceptar cualquier `n`, lo **snapea** al divisor exacto de 96 más cercano dentro de `{1,2,3,4,6,8,12,16,24}`. Nunca falla (no rechaza), pero el `n` efectivo puede no ser el pedido — `subscribe`/`set_subdivision` devuelven el valor resuelto para que el caller lo sepa.

`EveryNBars(n)` no tiene este problema nunca: `n × 96` siempre es entero.

## 4. C) `Clock` (el Observer) y `Subscription`

### 4.1 Por qué `&self` y no `&mut self`

Toda la API pública de `Clock` — incluido `advance` — usa `&self`, nunca `&mut self`. No es estilo, es necesidad: `Subscription<'clock>` guarda una referencia compartida `&'clock Clock` (puede haber varias vivas a la vez, una por secuenciador suscripto), y la plataforma necesita poder seguir llamando `clock.advance(&time)` en cada pump mientras esas `Subscription` siguen vivas en otros lados del programa. Si `advance` pidiera `&mut Clock`, el borrow checker no dejaría que ninguna `Subscription` siguiera viva al mismo tiempo — sería incompatible con tener secuenciadores de larga vida suscriptos.

La solución: todo el estado mutable de `Clock` vive detrás de `Cell`/`RefCell` (ambos en `core`, sin pedir `alloc`). Tanto la plataforma como cada `Subscription` acceden vía `&Clock` compartido. Es el patrón estándar para "un objeto de un solo hilo, con mutabilidad interior, prestado por muchos lados a la vez" — sin `unsafe` y sin necesitar `Rc` (nadie necesita *poseer* el `Clock`; todos lo prestan desde donde vive, una sola vez, en la plataforma).

### 4.2 Forma

```rust
pub const MAX_SUBS: usize = 32; // cubre el "al menos 20" del enunciado con margen

struct Slot {
    subdivision: Subdivision,
    interval_ticks: u32, // resuelto una vez al suscribirse / al cambiar subdivisión
    pending: u32,        // cuenta saturante: ticks debidos desde el último take_pending()
    generation: u32,
}

pub struct Clock {
    bpm: Cell<Bpm>,
    precision_mode: Cell<bool>,
    tick: Cell<u64>,                 // contador de ticks base (24 PPQN) desde el arranque
    next_tick_at_us: Cell<Option<u64>>,
    slots: RefCell<[Option<Slot>; MAX_SUBS]>,
    next_generation: Cell<u32>,
}

impl Clock {
    pub fn new(bpm: Bpm) -> Self { /* ... */ }

    /// Lee `time.now_us()`, avanza el contador de ticks según el período que
    /// da el bpm actual, y por cada tick base vencido marca `pending += 1` en
    /// cada slot cuyo intervalo calza. "Notificar a los grupos de cada tipo
    /// de subdivisión" es exactamente esto: no hay una estructura de
    /// agrupación separada — agrupar es que varios slots compartan el mismo
    /// intervalo resuelto y se marquen en el mismo tick.
    pub fn advance(&self, time: &Time) { /* ... */ }

    /// "Cambiar bpm global" (global al Clock, no al programa — ver §1.3).
    pub fn set_bpm(&self, bpm: Bpm) { self.bpm.set(bpm); }

    pub fn bpm(&self) -> Bpm { self.bpm.get() }

    pub fn set_precision_mode(&self, on: bool) { self.precision_mode.set(on); }

    /// Busca un slot libre en los MAX_SUBS; si no hay, falla explícito en vez
    /// de crecer dinámicamente (no hay alloc).
    pub fn subscribe(&self, subdivision: Subdivision) -> Result<Subscription<'_>, ClockFull> { /* ... */ }

    /// Para Debug (§6): el contador de ticks base crudo.
    pub fn tick(&self) -> u64 { self.tick.get() }
}

pub struct Subscription<'clock> {
    clock: &'clock Clock,
    index: usize,
    generation: u32,
}

impl<'clock> Subscription<'clock> {
    /// Lee y resetea a 0 el contador pending del slot. Devuelve cuántos
    /// ticks de esta subdivisión se acumularon desde la última lectura —
    /// nunca se pierde un paso aunque el consumidor lea más lento que los
    /// ticks (mismo principio que `pending_offs` ya usa en engine.rs).
    pub fn take_pending(&self) -> u32 { /* ... */ }

    /// Re-resuelve `interval_ticks` para la nueva subdivisión. Devuelve el
    /// valor efectivo aplicado (puede diferir del pedido en modo precisión).
    pub fn set_subdivision(&self, new: Subdivision) -> Result<Subdivision, SubscriptionError> { /* ... */ }

    /// "Eliminar suscripción" explícita, para quien no quiera esperar al Drop.
    pub fn unsubscribe(self) { /* consume self; el Drop de abajo hace el trabajo */ }
}

impl<'clock> Drop for Subscription<'clock> {
    /// Satisface el requisito de "al eliminarse el objeto suscriptor, se
    /// desuscribe solo" — ver §1.5. `try_borrow_mut` en vez de `borrow_mut`:
    /// un panic dentro de un Drop (p. ej. durante un unwind) es peor que
    /// simplemente no liberar el slot esa vez.
    fn drop(&mut self) {
        if let Ok(mut slots) = self.clock.slots.try_borrow_mut() {
            if let Some(slot) = &slots[self.index] {
                if slot.generation == self.generation {
                    slots[self.index] = None;
                }
            }
        }
    }
}
```

### 4.3 Operaciones del enunciado, mapeadas

| Pedido original | Dónde queda |
|---|---|
| Crear un clock | `Clock::new(bpm)` |
| Suscribirte a un clock, con subdivisión | `Clock::subscribe(subdivision)` |
| Cambiar subdivisión de una suscripción | `Subscription::set_subdivision(new)` |
| Desuscribirte | `Subscription::unsubscribe()` explícito, o automático al Drop |
| Notificar a los grupos de cada subdivisión | Dentro de `Clock::advance`, ver comentario en §4.2 |
| Cambiar bpm global | `Clock::set_bpm(bpm)` |
| Obtener mensajes/ticks pendientes | `Subscription::take_pending()` |

## 5. D) `Sequencer` — contrato, implementación diferida

No se toca `Engine`/`Pattern`/el parser MIDI actual en esta sesión — el enunciado pidió explícitamente dejar ese refactor para después. Lo único que se fija ahora es el contrato que cualquier secuenciador deberá cumplir para colgarse de un `Clock`:

```rust
pub trait Sequencer {
    fn on_ticks(&mut self, ticks: u32, out: &mut Vec<TimedMidi>);
}
```

Uso esperado, del lado de la plataforma, por cada secuenciador suscripto:

```rust
if let Some(n) = NonZeroU32::new(sub.take_pending()) {
    seq.on_ticks(n.get(), out);
}
```

Esto habilita D) tal como lo pide el enunciado: varios `Sequencer` colgados del mismo `Clock`, cada uno con su propia `Subscription`/subdivisión, compartiendo un único `Bpm` (el del `Clock`). `Vec<TimedMidi>` en la firma es deliberado: es la misma forma que ya usa `Engine::advance(now, lookahead, out)` hoy — el refactor futuro de `Engine` para implementar este trait no debería tener que cambiar cómo emite eventos, solo de dónde saca el conteo de ticks.

**Fuera de alcance acá, explícitamente para después:** la migración de `Engine` para que implemente `Sequencer`; el reemplazo del parser MIDI casero (`midi.rs`) por una librería externa — ambos mencionados en el enunciado como trabajo posterior, no de esta sesión.

## 6. E) `Debug`

```rust
pub struct TimeMark {
    pub bar: u32,
    pub beat: u8,
    pub tick_in_beat: u8,
    pub at_us: u64,
}

/// El core nunca llama `println!`/`std::io` directo — rompería `no_std`.
/// Cada plataforma inyecta su propio sink.
pub trait DebugSink {
    fn emit(&mut self, mark: TimeMark);
}
```

`Debug` se suscribe como cualquier otro consumidor (tiene su propia `Subscription`, típicamente `PerBar(1)` para una marca por bar, o más fino si se quiere ver subdivisiones), y en cada `take_pending() > 0` traduce `clock.tick()` a bar/beat/tick-in-beat (asumiendo 4/4 — ver §7) y lo empuja al `DebugSink` inyectado.

Sinks esperados por plataforma: en tests de host, uno que junta todo en un `Vec<TimeMark>` para assertions; en la web, uno que hace `console.log`; en ESP32, uno que hace `log::info!` sobre UART0 (ya existe ese canal, ver `docs/product/usb-midi-hardware.md`).

## 7. Preguntas que quedaban abiertas en el borrador original, resueltas acá

- **"Time es un singleton... los clocks se derivan de leer el ___"** (frase cortada en el original): se completa en §1.2 — los `Clock` derivan de leer `Time`, no la poseen.
- **"Bpm puede haber múltiples" vs. "1 único bpm o clock preciso por instancia de observer"**: no son contradictorios una vez que `Bpm` vive en el `Clock`, no en `Time` — ver §1.3.
- **"Requisitos: Async"**: resuelto como "no bloqueante" vía el modelo pull, sin `async/await` — ver §1.6.
- **"Algún macro" para auto-desuscripción**: resuelto con `Drop` — ver §1.5.
- **Time signature**: fijo en 4/4 para esta versión (afecta `ticks_per_bar = 96` en §3 y el cálculo de bar/beat en §6). Es una simplificación deliberada para no bloquear el resto del diseño, no un límite permanente — queda en "fuera de alcance" abajo.

## 8. Fuera de alcance (explícitamente, para no perderlo)

- Implementación real en `crates/obrero-core` — próxima sesión.
- Migración `no_std` del resto del crate (hoy `engine.rs`/`pattern.rs` compilan contra `std` sin declararlo — ver `docs/INDEX.md` §5). El código nuevo de este spec debe nacer `no_std`-limpio; retrofittear el resto es trabajo aparte.
- Múltiples `Clock` simultáneos — el diseño ya no lo impide (cada uno con su `Bpm`, todos podrían leer la misma `Time`), pero no se construye en esta sesión.
- Reemplazo de `midi.rs` por una librería externa (parte de D, diferido).
- Bindings JS de la superficie pública (`wasm-bindgen`) — igual que en el borrador original, queda como placeholder hasta que haya implementación.
- Time signatures distintas de 4/4.

## 9. Plan de testing (una vez implementado)

- **Precisión/deriva**: análogo a `no_drift_over_simulated_minutes` (`crates/obrero-core/tests/engine.rs`), pero sobre `Subscription::take_pending()` — correr minutos simulados y verificar que el conteo acumulado de ticks por subdivisión no se desvía del esperado.
- **Escala**: `MAX_SUBS` (32) suscripciones simultáneas, mezclando `PerBar`/`EveryNBars`, verificar que todas reciben su `pending` correcto en el mismo `advance`.
- **Ciclo de vida**: el `Drop` de una `Subscription` libera su slot; la suscripción número `MAX_SUBS + 1` falla antes de liberar una, y funciona después de que se libera una.
- **Modo precisión**: `PerBar(n)` con `n` no divisor de 96 se resuelve al divisor esperado cuando el modo está activo, y a la distribución euclidiana cuando no.

## 10. Próximos pasos

1. Implementar A–C en `crates/obrero-core` (nuevo módulo, p. ej. `clock.rs`), con los tests de §9.
2. Confirmar que compila `no_std` (añadir el check a CI, ver `.github/workflows/rust_ci.yml`).
3. Refactorizar `Engine` para implementar el contrato `Sequencer` (D) — sesión separada.
4. `Debug` (E) con al menos un sink de host para tests, antes de pensar en los sinks de plataforma.
