use std::cell::Cell;
use std::rc::Rc;

use obrero_core::clock::{
    Bpm, Clock, ClockError, DebugSink, DebugTap, Notice, Subdivision, Subscription, Time, TimeMark,
    TimeSource, MAX_SUBS, NOTICE_CAPACITY, PER_BAR_MAX, TICKS_PER_BAR,
};
use obrero_core::pattern::PPQN;

struct FakeTime(Cell<u64>);

impl TimeSource for FakeTime {
    fn now_us(&self) -> u64 {
        self.0.get()
    }
}

/// Instante exacto del tick n a `centi` bpm, anclado en 0.
fn exact_us(tick: u64, centi: u64) -> u64 {
    (tick as u128 * 6_000_000_000 / (centi as u128 * PPQN as u128)) as u64
}

/// Bombea como la web: `window_us` entre pumps, lookahead fijo, y retira
/// los avisos de `sub` después de cada advance.
fn pump(
    clock: &Rc<Clock>,
    sub: &Subscription,
    until_us: u64,
    window_us: u64,
    lookahead_us: u64,
) -> Vec<Notice> {
    let source = FakeTime(Cell::new(0));
    let mut time = Time::new();
    let mut all = Vec::new();
    while source.now_us() < until_us {
        time.advance(source.now_us());
        clock.advance(&time, lookahead_us);
        assert_eq!(sub.take(&mut all), 0, "no debe perder avisos");
        source.0.set(source.now_us() + window_us);
    }
    all
}

fn at(now_us: u64) -> Time {
    let mut t = Time::new();
    t.advance(now_us);
    t
}

#[test]
fn tick_times_are_exact_at_120bpm() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(24)).unwrap();
    let notices = pump(&clock, &sub, 2_000_000, 25_000, 100_000);
    assert!(!notices.is_empty());
    for n in &notices {
        assert_eq!(n.at_us, exact_us(n.tick, 12000), "tick {}", n.tick);
        assert_eq!(n.tick % 80, 0);
    }
}

#[test]
fn no_drift_over_simulated_minutes() {
    // 5 minutos a 120 bpm en ventanas de 25 ms con 100 ms de lookahead (web).
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(4)).unwrap();
    let notices = pump(&clock, &sub, 300_000_000, 25_000, 100_000);

    for pair in notices.windows(2) {
        assert_eq!(
            pair[1].tick - pair[0].tick,
            PPQN as u64,
            "ni saltos ni duplicados"
        );
    }
    let last = notices.last().unwrap();
    assert_eq!(last.at_us, exact_us(last.tick, 12000));
    // Último pump en 299.975 s + 100 ms de lookahead: negras hasta 300.075 s.
    assert_eq!(notices.len() as u64, 300_075_000 / 500_000 + 1);
}

#[test]
fn no_drift_over_one_hour_with_coarse_windows() {
    let clock = Clock::new(Bpm::from_centi(13_337));
    let sub = clock.subscribe(Subdivision::PerBar(1)).unwrap();
    let notices = pump(&clock, &sub, 3_600_000_000, 80_000, 0);
    for n in &notices {
        assert_eq!(n.at_us, exact_us(n.tick, 13_337));
    }
    for pair in notices.windows(2) {
        assert_eq!(pair[1].tick - pair[0].tick, TICKS_PER_BAR as u64);
    }
}

#[test]
fn overlapping_windows_never_duplicate_notices() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(24)).unwrap();
    let mut all = Vec::new();
    for _ in 0..5 {
        clock.advance(&at(0), 100_000);
        sub.take(&mut all);
    }
    clock.advance(&at(50_000), 100_000);
    sub.take(&mut all);
    let ticks: Vec<u64> = all.iter().map(|n| n.tick).collect();
    let mut dedup = ticks.clone();
    dedup.dedup();
    assert_eq!(ticks, dedup);
    assert_eq!(ticks, vec![0, 80]);
}

#[test]
fn tempo_change_applies_only_to_future_ticks_without_jumps() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(24)).unwrap();
    let mut before = Vec::new();
    clock.advance(&at(0), 100_000);
    sub.take(&mut before);
    for n in &before {
        assert_eq!(n.at_us, exact_us(n.tick, 12000), "lo ya emitido no cambia");
    }

    clock.set_bpm(Bpm::from_centi(15000));
    assert_eq!(clock.bpm(), Bpm::from_centi(15000));
    let mut all = before.clone();
    let mut now = 0;
    while now < 1_000_000 {
        now += 25_000;
        clock.advance(&at(now), 100_000);
        sub.take(&mut all);
    }
    for pair in all.windows(2) {
        let d = pair[1].at_us - pair[0].at_us;
        assert!(
            (66_666..=83_334).contains(&d),
            "intervalo fuera de rango: {d}"
        );
    }
    let tail: Vec<u64> = all[all.len() - 5..]
        .windows(2)
        .map(|p| p[1].at_us - p[0].at_us)
        .collect();
    assert!(
        tail.iter().all(|d| (66_666..=66_667).contains(d)),
        "{tail:?}"
    );
}

#[test]
fn per_bar_flexible_spreads_seven_notices_per_bar() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(7)).unwrap();
    let notices = pump(&clock, &sub, 4_000_000, 25_000, 100_000);
    let bar = TICKS_PER_BAR as u64;
    let bar0: Vec<u64> = notices
        .iter()
        .map(|n| n.tick)
        .filter(|t| *t < bar)
        .collect();
    let bar1: Vec<u64> = notices
        .iter()
        .map(|n| n.tick)
        .filter(|t| (bar..2 * bar).contains(t))
        .collect();
    assert_eq!(bar0.len(), 7);
    assert_eq!(bar1.len(), 7);
    assert_eq!(bar0[0], 0);
    for pair in bar0.windows(2) {
        assert!((274..=275).contains(&(pair[1] - pair[0])));
    }
}

#[test]
fn precision_mode_snaps_to_divisors_of_the_bar_and_reports_it() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let seven = clock.subscribe(Subdivision::PerBar(7)).unwrap();
    assert_eq!(seven.subdivision(), Subdivision::PerBar(7));

    clock.set_precision_mode(true);
    assert_eq!(seven.subdivision(), Subdivision::PerBar(6));
    let five = clock.subscribe(Subdivision::PerBar(5)).unwrap();
    assert_eq!(five.subdivision(), Subdivision::PerBar(5), "5 divide 1920");
    assert_eq!(
        five.set_subdivision(Subdivision::PerBar(9)),
        Ok(Subdivision::PerBar(8)),
        "empate entre 8 y 10: el menor"
    );
    assert_eq!(
        five.set_subdivision(Subdivision::PerBar(14)),
        Ok(Subdivision::PerBar(15))
    );
    assert_eq!(
        five.set_subdivision(Subdivision::PerBar(PER_BAR_MAX)),
        Ok(Subdivision::PerBar(96))
    );
    assert_eq!(
        five.set_subdivision(Subdivision::EveryNBars(3)),
        Ok(Subdivision::EveryNBars(3))
    );

    clock.set_precision_mode(false);
    assert_eq!(seven.subdivision(), Subdivision::PerBar(7));
}

#[test]
fn every_n_bars_fires_at_bar_starts() {
    let clock = Clock::new(Bpm::from_centi(30000));
    let sub = clock.subscribe(Subdivision::EveryNBars(8)).unwrap();
    // 17 bars a 300 bpm = 13.6 s.
    let notices = pump(&clock, &sub, 13_600_000, 25_000, 0);
    let ticks: Vec<u64> = notices.iter().map(|n| n.tick).collect();
    let bars = TICKS_PER_BAR as u64 * 8;
    assert_eq!(ticks, vec![0, bars, 2 * bars]);
}

#[test]
fn out_of_range_subdivisions_are_rejected() {
    let clock = Clock::new(Bpm::from_centi(12000));
    for bad in [
        Subdivision::PerBar(0),
        Subdivision::PerBar(PER_BAR_MAX + 1),
        Subdivision::EveryNBars(0),
        Subdivision::EveryNBars(9),
    ] {
        assert_eq!(clock.subscribe(bad).err(), Some(ClockError::OutOfRange));
    }
    let ok = clock.subscribe(Subdivision::PerBar(4)).unwrap();
    assert_eq!(
        ok.set_subdivision(Subdivision::PerBar(200)),
        Err(ClockError::OutOfRange)
    );
    assert_eq!(ok.subdivision(), Subdivision::PerBar(4));
}

#[test]
fn max_subscriptions_all_notified_in_the_same_advance() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let subs: Vec<(Subdivision, Subscription)> = (0..MAX_SUBS)
        .map(|i| {
            let s = if i % 2 == 0 {
                Subdivision::PerBar((i % 24 + 1) as u8 * 4)
            } else {
                Subdivision::EveryNBars((i % 8 + 1) as u8)
            };
            (s, clock.subscribe(s).unwrap())
        })
        .collect();

    let mut got = vec![0u64; MAX_SUBS];
    let mut buf = Vec::new();
    let mut now = 0;
    // 4 bars; se cuentan solo los avisos de esos ticks.
    let limit = 4 * TICKS_PER_BAR as u64;
    while clock.tick() < limit {
        clock.advance(&at(now), 100_000);
        for (i, (_, sub)) in subs.iter().enumerate() {
            assert_eq!(sub.take(&mut buf), 0);
            got[i] += buf.drain(..).filter(|n| n.tick < limit).count() as u64;
        }
        now += 25_000;
    }
    for (i, (s, _)) in subs.iter().enumerate() {
        let expected = match *s {
            Subdivision::PerBar(n) => 4 * n as u64,
            Subdivision::EveryNBars(m) => 4u64.div_ceil(m as u64),
            Subdivision::EveryTick => unreachable!(),
        };
        assert_eq!(got[i], expected, "suscripción {i} ({s:?})");
    }
}

#[test]
fn full_clock_rejects_until_a_subscription_is_dropped() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let mut subs: Vec<Subscription> = (0..MAX_SUBS)
        .map(|_| clock.subscribe(Subdivision::PerBar(1)).unwrap())
        .collect();
    assert_eq!(
        clock.subscribe(Subdivision::PerBar(1)).err(),
        Some(ClockError::Full)
    );

    drop(subs.pop());
    let again = clock.subscribe(Subdivision::PerBar(1)).unwrap();
    subs.pop().unwrap().unsubscribe();
    let third = clock.subscribe(Subdivision::PerBar(1)).unwrap();

    clock.advance(&at(0), 0);
    let mut buf = Vec::new();
    again.take(&mut buf);
    third.take(&mut buf);
    assert_eq!(buf.len(), 2, "los slots reusados reciben avisos propios");
}

#[test]
fn reused_slot_starts_empty() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let old = clock.subscribe(Subdivision::PerBar(24)).unwrap();
    clock.advance(&at(0), 100_000);
    drop(old);
    let new = clock.subscribe(Subdivision::PerBar(24)).unwrap();
    let mut buf = Vec::new();
    assert_eq!(new.take(&mut buf), 0);
    assert!(
        buf.is_empty(),
        "no hereda avisos de la suscripción anterior"
    );
}

#[test]
fn overflow_is_counted_not_silent() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock
        .subscribe_with_capacity(Subdivision::PerBar(24), 16)
        .unwrap();
    // 2 bars de lookahead sin leer: 48 avisos para un ring de 16.
    clock.advance(&at(0), 3_990_000);
    let mut buf = Vec::new();
    assert_eq!(sub.take(&mut buf), 32);
    assert_eq!(buf.len(), 16);
    assert_eq!(buf[0].tick, 0);
    assert_eq!(sub.take(&mut buf), 0, "el contador se resetea al leer");
}

#[test]
fn large_stall_skips_ahead_keeping_bar_alignment() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(4)).unwrap();
    clock.advance(&at(0), 0);
    let mut buf = Vec::new();
    sub.take(&mut buf);

    // 10 s sin bombear: no hay ráfaga de 20 negras vencidas.
    clock.advance(&at(10_000_000), 100_000);
    buf.clear();
    assert_eq!(sub.take(&mut buf), 0);
    assert!(clock.skipped_ticks() > 0);
    assert_eq!(buf.len(), 1, "solo la negra de los 10 s");
    for n in &buf {
        assert_eq!(n.at_us, exact_us(n.tick, 12000), "sigue en la misma grilla");
        assert!(n.at_us >= 9_900_000);
    }
}

#[test]
fn reset_reanchors_tick_zero_and_drops_pending() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(4)).unwrap();
    clock.advance(&at(0), 1_000_000);
    clock.reset();
    assert_eq!(clock.tick(), 0);
    clock.advance(&at(5_000_000), 0);
    let mut buf = Vec::new();
    sub.take(&mut buf);
    assert_eq!(
        buf,
        vec![Notice {
            tick: 0,
            at_us: 5_000_000
        }]
    );
}

#[test]
fn time_never_goes_backwards() {
    let mut t = Time::new();
    t.advance(1_000);
    t.advance(500);
    assert_eq!(t.now_us(), 1_000);
}

#[test]
fn bpm_is_clamped_and_nan_safe() {
    assert_eq!(Bpm::from_centi(0), Bpm::MIN);
    assert_eq!(Bpm::from_centi(u32::MAX), Bpm::MAX);
    assert_eq!(Bpm::from_f32(f32::NAN), Bpm::MIN);
    assert_eq!(Bpm::from_f32(1000.0), Bpm::MAX);
    assert_eq!(Bpm::from_f32(120.004).centi(), 12000);
    assert_eq!(Bpm::from_f32(133.37).centi(), 13337);
    assert_eq!(Bpm::from_centi(12050).as_f32(), 120.5);
}

struct VecSink(Vec<TimeMark>);

impl DebugSink for VecSink {
    fn emit(&mut self, mark: TimeMark) {
        self.0.push(mark);
    }
}

#[test]
fn debug_tap_reports_bar_and_beat() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let mut tap = DebugTap::new(&clock, Subdivision::PerBar(4)).unwrap();
    let mut sink = VecSink(Vec::new());
    let mut now = 0;
    while clock.tick() < 2 * TICKS_PER_BAR as u64 {
        clock.advance(&at(now), 100_000);
        assert_eq!(tap.poll(&mut sink), 0);
        now += 25_000;
    }
    let marks: Vec<(u32, u8, u16)> = sink
        .0
        .iter()
        .filter(|m| m.bar < 2)
        .map(|m| (m.bar, m.beat, m.tick_in_beat))
        .collect();
    assert_eq!(
        marks,
        vec![
            (0, 0, 0),
            (0, 1, 0),
            (0, 2, 0),
            (0, 3, 0),
            (1, 0, 0),
            (1, 1, 0),
            (1, 2, 0),
            (1, 3, 0)
        ]
    );
    assert_eq!(sink.0[1].at_us, 500_000);
}

#[test]
fn ring_capacity_is_rounded_up_to_a_power_of_two() {
    let clock = Clock::new(Bpm::from_centi(12000));
    // 0 -> 1, 5 -> 8: se aceptan 8 avisos y el noveno se cuenta como perdido.
    let one = clock
        .subscribe_with_capacity(Subdivision::EveryTick, 0)
        .unwrap();
    let five = clock
        .subscribe_with_capacity(Subdivision::EveryTick, 5)
        .unwrap();
    clock.advance(&at(0), 10_000); // ~10 ticks
    let mut buf = Vec::new();
    assert_eq!(five.take(&mut buf), 2);
    assert_eq!(buf.len(), 8);
    buf.clear();
    assert_eq!(one.take(&mut buf) as usize + buf.len(), 10);
    assert_eq!(buf.len(), 1);
    assert_eq!(
        clock
            .subscribe_with_capacity(Subdivision::EveryTick, usize::MAX)
            .err(),
        Some(ClockError::OutOfRange)
    );
}

#[test]
fn default_rings_are_powers_of_two_sized_for_the_worst_case() {
    use obrero_core::clock::ENGINE_RING_CAPACITY;
    assert!(NOTICE_CAPACITY.is_power_of_two());
    assert!(ENGINE_RING_CAPACITY.is_power_of_two());
    assert_eq!((NOTICE_CAPACITY, ENGINE_RING_CAPACITY), (32, 512));
}

#[test]
fn every_tick_gets_each_base_tick_once_with_exact_times() {
    let clock = Clock::new(Bpm::from_centi(30000));
    let sub = clock
        .subscribe_with_capacity(Subdivision::EveryTick, 2048)
        .unwrap();
    let notices = pump(&clock, &sub, 5_000_000, 25_000, 100_000);
    for (i, n) in notices.iter().enumerate() {
        assert_eq!(n.tick, i as u64, "ni saltos ni duplicados");
        assert_eq!(n.at_us, exact_us(n.tick, 30000));
    }
    assert!(notices.len() > 10_000);
}

#[test]
fn default_ring_covers_worst_case_pass_without_loss() {
    // Peor caso: atraso máximo recuperable + lookahead máximo, a MAX_BPM,
    // con la subdivisión más fina permitida.
    let clock = Clock::new(Bpm::MAX);
    let sub = clock.subscribe(Subdivision::PerBar(PER_BAR_MAX)).unwrap();
    clock.advance(&at(0), 0);
    let mut buf = Vec::new();
    sub.take(&mut buf);
    buf.clear();
    clock.advance_to(
        obrero_core::clock::MAX_CATCHUP_US,
        obrero_core::clock::MAX_CATCHUP_US + obrero_core::clock::MAX_LOOKAHEAD_US,
    );
    assert_eq!(sub.take(&mut buf), 0);
    assert!(buf.len() <= NOTICE_CAPACITY);
}

#[test]
fn next_tick_at_us_tracks_the_ungenerated_tick() {
    let clock = Clock::new(Bpm::from_centi(12000));
    assert_eq!(clock.next_tick_at_us(), None);
    clock.advance(&at(1_000_000), 0);
    assert_eq!(clock.tick(), 1);
    assert_eq!(
        clock.next_tick_at_us(),
        Some(1_000_000 + exact_us(1, 12000))
    );
}

fn external(clock: &Rc<Clock>, pulses_at: impl Iterator<Item = u64>) {
    for t in pulses_at {
        clock.external_pulse(t);
    }
}

#[test]
fn external_pulse_spreads_ticks_evenly_inside_each_pulse() {
    use obrero_core::clock::ClockSource;
    let clock = Clock::new(Bpm::from_centi(12000));
    clock.set_source(ClockSource::External);
    let sub = clock
        .subscribe_with_capacity(Subdivision::EveryTick, 128)
        .unwrap();
    // 120 bpm = 0xF8 cada 20 833,33 µs; se mide el período con el 2.º pulso.
    external(&clock, (0..4).map(|i| i * 20_833));
    let mut n = Vec::new();
    assert_eq!(sub.take(&mut n), 0);
    assert_eq!(n.len(), 80);
    for (i, notice) in n.iter().enumerate() {
        assert_eq!(notice.tick, i as u64);
    }
    for pair in n.windows(2) {
        assert!(pair[1].at_us >= pair[0].at_us, "estampas no decrecientes");
    }
    // 3.er pulso (ticks 40..60): arranca en su llegada y avanza ~1041 µs por tick.
    assert_eq!(n[40].at_us, 2 * 20_833);
    let d = n[41].at_us - n[40].at_us;
    assert!((1_040..=1_042).contains(&d), "{d}");
}

#[test]
fn external_pulse_measures_bpm_and_follows_a_tempo_ramp() {
    use obrero_core::clock::ClockSource;
    let clock = Clock::new(Bpm::from_centi(12000));
    clock.set_source(ClockSource::External);
    let mut t = 0u64;
    for _ in 0..200 {
        clock.external_pulse(t);
        t += 25_000; // 100 bpm exactos
    }
    let bpm = clock.bpm().centi();
    assert!((9_990..=10_010).contains(&bpm), "{bpm}");
    for _ in 0..200 {
        clock.external_pulse(t);
        t += 20_833; // 120 bpm
    }
    let bpm = clock.bpm().centi();
    assert!((11_980..=12_020).contains(&bpm), "{bpm}");
}

#[test]
fn external_pulse_filters_jitter_without_drift() {
    use obrero_core::clock::ClockSource;
    let clock = Clock::new(Bpm::from_centi(12000));
    clock.set_source(ClockSource::External);
    let sub = clock
        .subscribe_with_capacity(Subdivision::EveryTick, 64)
        .unwrap();
    let mut notices = Vec::new();
    // 5 minutos a 120 bpm con jitter de ±2 ms por pulso (determinista).
    let mut seed = 12345u64;
    for i in 0..14_400u64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let jitter = (seed >> 33) % 4_001;
        clock.external_pulse(i * 20_833 + jitter);
        sub.take(&mut notices);
    }
    assert_eq!(notices.len(), 14_400 * 20);
    let bpm = clock.bpm().centi();
    assert!((11_950..=12_050).contains(&bpm), "{bpm}");
    for pair in notices.windows(2) {
        assert!(pair[1].at_us >= pair[0].at_us);
        assert_eq!(pair[1].tick, pair[0].tick + 1);
    }
}

#[test]
fn external_silence_is_a_cut_not_a_tempo_and_internal_ignores_pulses() {
    use obrero_core::clock::ClockSource;
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock
        .subscribe_with_capacity(Subdivision::EveryTick, 64)
        .unwrap();
    clock.external_pulse(0); // en modo interno no hace nada
    let mut n = Vec::new();
    sub.take(&mut n);
    assert!(n.is_empty());

    clock.set_source(ClockSource::External);
    external(&clock, (0..30).map(|i| i * 20_833));
    let before = clock.bpm().centi();
    clock.external_pulse(30 * 20_833 + 10_000_000); // 10 s de silencio
    assert_eq!(clock.bpm().centi(), before);
    // En modo externo `advance` no genera ticks por su cuenta.
    let ticks = clock.tick();
    clock.advance(&at(50_000_000), 100_000);
    assert_eq!(clock.tick(), ticks);
}
