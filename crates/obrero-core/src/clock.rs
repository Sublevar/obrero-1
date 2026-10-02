//! Time / Bpm / Clock / Subscription / Debug — ver `specs/clock.md`.
//!
//! Observer pull: `Clock::advance` solo encola avisos con timestamp en cada
//! suscripción; nunca ejecuta código ajeno. Cada consumidor los retira cuando
//! a él lo hacen avanzar.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use crate::engine::{TimedMidi, MAX_BPM, MIN_BPM};
use crate::pattern::{euclidean_hit, PPQN};

pub const BEATS_PER_BAR: u32 = 4;
pub const TICKS_PER_BAR: u32 = PPQN * BEATS_PER_BAR;
pub const MAX_SUBS: usize = 32;
/// Avisos retenidos por suscripción. 100 ms de lookahead a 300 bpm son 12
/// ticks base: con 16 alcanza si el consumidor lee en cada pump.
pub const NOTICE_CAPACITY: usize = 16;
pub const PER_BAR_MAX: u8 = 24;
pub const EVERY_N_BARS_MAX: u8 = 8;
/// Atraso máximo que `advance` recupera tick por tick. Más allá (pestaña
/// dormida, laptop suspendida) salta hacia adelante en vez de emitir una
/// ráfaga de ticks vencidos, y lo informa en `Clock::skipped_ticks`.
pub const MAX_CATCHUP_US: u64 = 1_000_000;

const US_PER_MINUTE_CENTI: u128 = 60_000_000 * 100;
const PRECISE_PER_BAR: [u8; 9] = [1, 2, 3, 4, 6, 8, 12, 16, 24];

/// Lectura del reloj real. Cada plataforma implementa la suya; el core no.
pub trait TimeSource {
    fn now_us(&self) -> u64;
}

/// Marca temporal monotónica del programa, en µs. Una por programa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Time {
    now_us: u64,
}

impl Time {
    pub const fn new() -> Self {
        Self { now_us: 0 }
    }

    /// Satura hacia adelante: un valor atrasado no hace retroceder el tiempo.
    pub fn advance(&mut self, now_us: u64) {
        self.now_us = self.now_us.max(now_us);
    }

    pub fn now_us(&self) -> u64 {
        self.now_us
    }
}

/// Tempo en centésimas de bpm (12000 = 120.00). Entero para que el cálculo
/// de tiempos sea exacto e idéntico en xtensa (sin FPU de 64 bits) y wasm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bpm(u32);

impl Bpm {
    pub const MIN: Bpm = Bpm(MIN_BPM as u32 * 100);
    pub const MAX: Bpm = Bpm(MAX_BPM as u32 * 100);

    pub const fn from_centi(centi: u32) -> Self {
        if centi < Self::MIN.0 {
            Self::MIN
        } else if centi > Self::MAX.0 {
            Self::MAX
        } else {
            Bpm(centi)
        }
    }

    /// NaN cae en el mínimo.
    pub fn from_f32(bpm: f32) -> Self {
        if bpm.is_nan() {
            return Self::MIN;
        }
        let bpm = bpm.clamp(MIN_BPM, MAX_BPM);
        Self::from_centi((bpm * 100.0 + 0.5) as u32)
    }

    pub const fn centi(self) -> u32 {
        self.0
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32 / 100.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subdivision {
    /// n avisos por bar, repartidos E(n, 96). 1..=24.
    PerBar(u8),
    /// 1 aviso cada n bars, al inicio del bar. 1..=8.
    EveryNBars(u8),
}

impl Subdivision {
    fn validate(self) -> Result<Self, ClockError> {
        let ok = match self {
            Subdivision::PerBar(n) => (1..=PER_BAR_MAX).contains(&n),
            Subdivision::EveryNBars(n) => (1..=EVERY_N_BARS_MAX).contains(&n),
        };
        if ok {
            Ok(self)
        } else {
            Err(ClockError::OutOfRange)
        }
    }

    /// Divisor exacto de 96 más cercano; ante empate, el menor.
    fn snap_precise(self) -> Self {
        match self {
            Subdivision::PerBar(n) => {
                let mut best = PRECISE_PER_BAR[0];
                for d in PRECISE_PER_BAR {
                    if d.abs_diff(n) < best.abs_diff(n) {
                        best = d;
                    }
                }
                Subdivision::PerBar(best)
            }
            every => every,
        }
    }

    fn resolve(self, precise: bool) -> Self {
        if precise {
            self.snap_precise()
        } else {
            self
        }
    }

    /// ¿El tick base `tick` lleva aviso? Alineado al compás: tick 0 = bar 0.
    pub fn hits(self, tick: u64) -> bool {
        match self {
            Subdivision::PerBar(n) => euclidean_hit(
                (tick % TICKS_PER_BAR as u64) as u32,
                n as u32,
                TICKS_PER_BAR,
            ),
            Subdivision::EveryNBars(n) => tick.is_multiple_of(TICKS_PER_BAR as u64 * n as u64),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockError {
    /// Los `MAX_SUBS` slots están ocupados.
    Full,
    /// Subdivisión fuera de `1..=PER_BAR_MAX` / `1..=EVERY_N_BARS_MAX`.
    OutOfRange,
}

/// Un aviso: el tick base que le tocó a la suscripción y su instante exacto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Notice {
    pub tick: u64,
    pub at_us: u64,
}

const NO_NOTICE: Notice = Notice { tick: 0, at_us: 0 };

struct Slot {
    requested: Subdivision,
    effective: Subdivision,
    generation: u32,
    ring: [Notice; NOTICE_CAPACITY],
    head: usize,
    len: usize,
    lost: u32,
}

impl Slot {
    /// Ring lleno: se descarta el aviso nuevo y se cuenta, nunca en silencio.
    fn push(&mut self, n: Notice) {
        if self.len == NOTICE_CAPACITY {
            self.lost = self.lost.saturating_add(1);
            return;
        }
        self.ring[(self.head + self.len) % NOTICE_CAPACITY] = n;
        self.len += 1;
    }

    fn drain_into(&mut self, out: &mut Vec<Notice>) -> u32 {
        for i in 0..self.len {
            out.push(self.ring[(self.head + i) % NOTICE_CAPACITY]);
        }
        self.head = 0;
        self.len = 0;
        core::mem::take(&mut self.lost)
    }
}

/// Punto desde el que se calculan los tiempos: el tick `tick` cae en `at_us`
/// y los siguientes a `bpm`. Cambiar el tempo crea un ancla nueva; nunca se
/// acumula un período redondeado.
#[derive(Clone, Copy, Debug)]
struct Anchor {
    tick: u64,
    at_us: u64,
    bpm: Bpm,
}

impl Anchor {
    fn time_of(self, tick: u64) -> u64 {
        let dt =
            (tick - self.tick) as u128 * US_PER_MINUTE_CENTI / (self.bpm.0 as u128 * PPQN as u128);
        self.at_us + dt as u64
    }

    /// Ticks enteros que caben en `us` a este tempo.
    fn ticks_in(self, us: u64) -> u64 {
        (us as u128 * self.bpm.0 as u128 * PPQN as u128 / US_PER_MINUTE_CENTI) as u64
    }
}

/// El Observer. Se usa siempre como `Rc<Clock>`: las suscripciones lo
/// co-poseen, así no llevan lifetime y pueden cruzar a JS.
pub struct Clock {
    bpm: Cell<Bpm>,
    precision_mode: Cell<bool>,
    anchor: Cell<Option<Anchor>>,
    next_tick: Cell<u64>,
    skipped: Cell<u64>,
    next_generation: Cell<u32>,
    slots: RefCell<[Option<Slot>; MAX_SUBS]>,
}

impl Clock {
    pub fn new(bpm: Bpm) -> Rc<Self> {
        Rc::new(Self {
            bpm: Cell::new(bpm),
            precision_mode: Cell::new(false),
            anchor: Cell::new(None),
            next_tick: Cell::new(0),
            skipped: Cell::new(0),
            next_generation: Cell::new(0),
            slots: RefCell::new([const { None }; MAX_SUBS]),
        })
    }

    /// Genera los ticks con instante <= now + lookahead y encola un aviso en
    /// cada suscripción cuya subdivisión cae en ese tick. La primera llamada
    /// (o la primera tras `reset`) ancla el tick 0 en `now`.
    pub fn advance(&self, time: &Time, lookahead_us: u64) {
        let now = time.now_us();
        let mut anchor = match self.anchor.get() {
            Some(a) => a,
            None => Anchor {
                tick: self.next_tick.get(),
                at_us: now,
                bpm: self.bpm.get(),
            },
        };
        let mut tick = self.next_tick.get();

        let due = anchor.time_of(tick);
        if now > due + MAX_CATCHUP_US {
            let skip = anchor.ticks_in(now - due);
            tick += skip;
            anchor = Anchor {
                tick,
                at_us: anchor.time_of(tick),
                bpm: anchor.bpm,
            };
            self.skipped.set(self.skipped.get() + skip);
        }

        let horizon = now.saturating_add(lookahead_us);
        // Ningún código ajeno corre mientras este borrow está vivo: es lo que
        // garantiza que el borrow del Drop de Subscription nunca falle.
        let mut slots = self.slots.borrow_mut();
        loop {
            let at_us = anchor.time_of(tick);
            if at_us > horizon {
                break;
            }
            for slot in slots.iter_mut().flatten() {
                if slot.effective.hits(tick) {
                    slot.push(Notice { tick, at_us });
                }
            }
            tick += 1;
        }
        self.next_tick.set(tick);
        self.anchor.set(Some(anchor));
    }

    /// Aplica desde el próximo tick todavía no generado: los ya emitidos en
    /// el lookahead conservan su instante.
    pub fn set_bpm(&self, bpm: Bpm) {
        self.bpm.set(bpm);
        if let Some(a) = self.anchor.get() {
            if a.bpm != bpm {
                let tick = self.next_tick.get();
                self.anchor.set(Some(Anchor {
                    tick,
                    at_us: a.time_of(tick),
                    bpm,
                }));
            }
        }
    }

    pub fn bpm(&self) -> Bpm {
        self.bpm.get()
    }

    /// Re-resuelve todas las suscripciones vivas desde lo que pidieron.
    pub fn set_precision_mode(&self, on: bool) {
        self.precision_mode.set(on);
        for slot in self.slots.borrow_mut().iter_mut().flatten() {
            slot.effective = slot.requested.resolve(on);
        }
    }

    pub fn precision_mode(&self) -> bool {
        self.precision_mode.get()
    }

    /// Vuelve al tick 0, anclado en el próximo `advance`. Descarta los avisos
    /// pendientes: pertenecen a la línea de tiempo anterior.
    pub fn reset(&self) {
        self.anchor.set(None);
        self.next_tick.set(0);
        for slot in self.slots.borrow_mut().iter_mut().flatten() {
            slot.head = 0;
            slot.len = 0;
        }
    }

    /// Próximo tick base a generar.
    pub fn tick(&self) -> u64 {
        self.next_tick.get()
    }

    /// Ticks salteados en total por atrasos mayores a `MAX_CATCHUP_US`.
    pub fn skipped_ticks(&self) -> u64 {
        self.skipped.get()
    }

    pub fn subscribe(
        self: &Rc<Self>,
        subdivision: Subdivision,
    ) -> Result<Subscription, ClockError> {
        let requested = subdivision.validate()?;
        let mut slots = self.slots.borrow_mut();
        let index = slots
            .iter()
            .position(Option::is_none)
            .ok_or(ClockError::Full)?;
        let generation = self.next_generation.get();
        self.next_generation.set(generation.wrapping_add(1));
        slots[index] = Some(Slot {
            requested,
            effective: requested.resolve(self.precision_mode.get()),
            generation,
            ring: [NO_NOTICE; NOTICE_CAPACITY],
            head: 0,
            len: 0,
            lost: 0,
        });
        Ok(Subscription {
            clock: Rc::clone(self),
            index,
            generation,
        })
    }
}

/// Suscripción viva a un `Clock`. Al soltarse libera su slot (el
/// "destructor unregisters" del spec §1.5).
pub struct Subscription {
    clock: Rc<Clock>,
    index: usize,
    generation: u32,
}

impl Subscription {
    fn with_slot<R>(&self, f: impl FnOnce(&mut Slot) -> R) -> Option<R> {
        let mut slots = self.clock.slots.borrow_mut();
        match &mut slots[self.index] {
            Some(slot) if slot.generation == self.generation => Some(f(slot)),
            _ => None,
        }
    }

    /// Mueve los avisos pendientes a `out`, en orden. Devuelve cuántos se
    /// perdieron por ring lleno desde la lectura anterior.
    pub fn take(&self, out: &mut Vec<Notice>) -> u32 {
        self.with_slot(|s| s.drain_into(out)).unwrap_or(0)
    }

    /// Subdivisión efectiva (puede diferir de la pedida en modo precisión).
    pub fn subdivision(&self) -> Subdivision {
        self.with_slot(|s| s.effective)
            .unwrap_or(Subdivision::PerBar(1))
    }

    /// Devuelve la subdivisión efectiva aplicada.
    pub fn set_subdivision(&self, new: Subdivision) -> Result<Subdivision, ClockError> {
        let requested = new.validate()?;
        let precise = self.clock.precision_mode.get();
        Ok(self
            .with_slot(|s| {
                s.requested = requested;
                s.effective = requested.resolve(precise);
                s.effective
            })
            .unwrap_or(requested))
    }

    pub fn clock(&self) -> &Rc<Clock> {
        &self.clock
    }

    pub fn unsubscribe(self) {}
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let Ok(mut slots) = self.clock.slots.try_borrow_mut() else {
            debug_assert!(false, "slots tomados durante el Drop de Subscription");
            return;
        };
        if matches!(&slots[self.index], Some(s) if s.generation == self.generation) {
            slots[self.index] = None;
        }
    }
}

/// Contrato para colgar un secuenciador de una `Subscription` (spec §5).
pub trait Sequencer {
    fn on_notices(&mut self, notices: &[Notice], out: &mut Vec<TimedMidi>);
}

/// Posición musical de un aviso, en 4/4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeMark {
    pub bar: u32,
    pub beat: u8,
    pub tick_in_beat: u8,
    pub at_us: u64,
}

impl From<Notice> for TimeMark {
    fn from(n: Notice) -> Self {
        let in_bar = (n.tick % TICKS_PER_BAR as u64) as u32;
        Self {
            bar: (n.tick / TICKS_PER_BAR as u64) as u32,
            beat: (in_bar / PPQN) as u8,
            tick_in_beat: (in_bar % PPQN) as u8,
            at_us: n.at_us,
        }
    }
}

pub trait DebugSink {
    fn emit(&mut self, mark: TimeMark);
}

/// Consumidor de depuración: traduce sus avisos a `TimeMark`.
pub struct DebugTap {
    sub: Subscription,
    buf: Vec<Notice>,
}

impl DebugTap {
    pub fn new(clock: &Rc<Clock>, subdivision: Subdivision) -> Result<Self, ClockError> {
        Ok(Self {
            sub: clock.subscribe(subdivision)?,
            buf: Vec::new(),
        })
    }

    /// Devuelve los avisos perdidos desde el poll anterior.
    pub fn poll(&mut self, sink: &mut impl DebugSink) -> u32 {
        let lost = self.sub.take(&mut self.buf);
        for n in self.buf.drain(..) {
            sink.emit(n.into());
        }
        lost
    }

    pub fn subscription(&self) -> &Subscription {
        &self.sub
    }
}
