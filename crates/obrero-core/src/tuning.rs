//! Variables de ajuste del core, todas en un solo lugar. Cada una está
//! documentada en la sección "Ajustes" del `README.md`.
//!
//! Los topes de ring se derivan de las demás constantes: al cambiar el PPQN,
//! el tempo máximo o las ventanas, los tamaños se recalculan solos y siguen
//! cubriendo el peor caso de un `advance`.

/// Ticks por negra. Múltiplo de `MIDI_CLOCK_PPQN`.
pub const PPQN: u32 = 480;
/// Resolución del MIDI clock (0xF8) que emite o recibe el motor. Fijada por el estándar.
pub const MIDI_CLOCK_PPQN: u32 = 24;
pub const TICKS_PER_MIDI_CLOCK: u32 = PPQN / MIDI_CLOCK_PPQN;
const _: () = assert!(TICKS_PER_MIDI_CLOCK * MIDI_CLOCK_PPQN == PPQN);

pub const BEATS_PER_BAR: u32 = 4;

pub const MIN_BPM: f32 = 20.0;
pub const MAX_BPM: f32 = 300.0;

/// Suscripciones simultáneas a un `Clock`.
pub const MAX_SUBS: usize = 32;
/// Tope de `Subdivision::PerBar`.
pub const PER_BAR_MAX: u8 = 96;
/// Tope de `Subdivision::EveryNBars`.
pub const EVERY_N_BARS_MAX: u8 = 8;

/// Atraso máximo que `Clock::advance` recupera tick por tick. Más allá
/// (pestaña dormida, laptop suspendida) salta hacia adelante sin ráfaga.
pub const MAX_CATCHUP_US: u64 = 100_000;
/// Lookahead máximo por pasada de `Clock::advance` que los rings cubren sin
/// desbordar. `Engine::advance` parte lookaheads mayores en pasadas de este
/// tamaño; quien use `Clock` directo debe respetarlo.
pub const MAX_LOOKAHEAD_US: u64 = 100_000;
/// Holgura sumada a los rings sobre el peor caso calculado, antes de
/// redondear a potencia de 2.
pub const RING_MARGIN: usize = 4;
/// Peso del último período medido en el clock externo: el período suavizado
/// es `(anterior · (N − 1) + medido) / N`. Más alto filtra más jitter y
/// sigue más lento los cambios de tempo.
pub const EXTERNAL_SMOOTHING: u64 = 4;

const US_PER_MINUTE_CENTI: u64 = 60_000_000 * 100;
const MAX_BPM_CENTI: u64 = MAX_BPM as u64 * 100;
const WORST_WINDOW_US: u64 = MAX_CATCHUP_US + MAX_LOOKAHEAD_US;

/// Ring por defecto de una suscripción: avisos de `PerBar(PER_BAR_MAX)` que
/// caen en una pasada, a `MAX_BPM`. Potencia de 2: el índice circular usa
/// máscara, sin división.
pub const NOTICE_CAPACITY: usize = ((WORST_WINDOW_US * MAX_BPM_CENTI * PER_BAR_MAX as u64
    / (US_PER_MINUTE_CENTI * BEATS_PER_BAR as u64)) as usize
    + 1
    + RING_MARGIN)
    .next_power_of_two();

/// Ring de la suscripción del `Engine`: todos los ticks que caen en una
/// pasada, a `MAX_BPM`. Potencia de 2.
pub const ENGINE_RING_CAPACITY: usize = ((WORST_WINDOW_US * MAX_BPM_CENTI * PPQN as u64
    / US_PER_MINUTE_CENTI) as usize
    + 1
    + RING_MARGIN)
    .next_power_of_two();

const _: () = assert!(NOTICE_CAPACITY.is_power_of_two() && ENGINE_RING_CAPACITY.is_power_of_two());
const _: () = assert!(ENGINE_RING_CAPACITY >= TICKS_PER_MIDI_CLOCK as usize);
