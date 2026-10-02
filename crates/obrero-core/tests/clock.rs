use std::cell::Cell;
use std::rc::Rc;

use obrero_core::clock::{
    Bpm, Clock, ClockError, DebugSink, DebugTap, Notice, Subdivision, Subscription, Time, TimeMark,
    TimeSource, MAX_SUBS, TICKS_PER_BAR,
};

struct FakeTime(Cell<u64>);

impl TimeSource for FakeTime {
    fn now_us(&self) -> u64 {
        self.0.get()
    }
}

/// Instante exacto del tick n a `centi` bpm, anclado en 0.
fn exact_us(tick: u64, centi: u64) -> u64 {
    (tick as u128 * 6_000_000_000 / (centi as u128 * 24)) as u64
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
        assert_eq!(n.tick % 4, 0);
    }
}

#[test]
fn no_drift_over_simulated_minutes() {
    // 5 minutos a 120 bpm en ventanas de 25 ms con 100 ms de lookahead (web).
    let clock = Clock::new(Bpm::from_centi(12000));
    let sub = clock.subscribe(Subdivision::PerBar(4)).unwrap();
    let notices = pump(&clock, &sub, 300_000_000, 25_000, 100_000);

    for pair in notices.windows(2) {
        assert_eq!(pair[1].tick - pair[0].tick, 24, "ni saltos ni duplicados");
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
    let notices = pump(&clock, &sub, 3_600_000_000, 500_000, 0);
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
    assert_eq!(ticks, vec![0, 4]);
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
    let bar0: Vec<u64> = notices.iter().map(|n| n.tick).filter(|t| *t < 96).collect();
    let bar1: Vec<u64> = notices
        .iter()
        .map(|n| n.tick)
        .filter(|t| (96..192).contains(t))
        .collect();
    assert_eq!(bar0.len(), 7);
    assert_eq!(bar1.len(), 7);
    assert_eq!(bar0[0], 0);
    for pair in bar0.windows(2) {
        assert!((13..=14).contains(&(pair[1] - pair[0])));
    }
}

#[test]
fn precision_mode_snaps_to_divisors_of_96_and_reports_it() {
    let clock = Clock::new(Bpm::from_centi(12000));
    let seven = clock.subscribe(Subdivision::PerBar(7)).unwrap();
    assert_eq!(seven.subdivision(), Subdivision::PerBar(7));

    clock.set_precision_mode(true);
    assert_eq!(seven.subdivision(), Subdivision::PerBar(6));
    let five = clock.subscribe(Subdivision::PerBar(5)).unwrap();
    assert_eq!(
        five.subdivision(),
        Subdivision::PerBar(4),
        "empate: el menor"
    );
    assert_eq!(
        five.set_subdivision(Subdivision::PerBar(10)),
        Ok(Subdivision::PerBar(8))
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
    assert_eq!(ticks, vec![0, 768, 1536]);
}

#[test]
fn out_of_range_subdivisions_are_rejected() {
    let clock = Clock::new(Bpm::from_centi(12000));
    for bad in [
        Subdivision::PerBar(0),
        Subdivision::PerBar(25),
        Subdivision::EveryNBars(0),
        Subdivision::EveryNBars(9),
    ] {
        assert_eq!(clock.subscribe(bad).err(), Some(ClockError::OutOfRange));
    }
    let ok = clock.subscribe(Subdivision::PerBar(4)).unwrap();
    assert_eq!(
        ok.set_subdivision(Subdivision::PerBar(30)),
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
                Subdivision::PerBar((i % 24 + 1) as u8)
            } else {
                Subdivision::EveryNBars((i % 8 + 1) as u8)
            };
            (s, clock.subscribe(s).unwrap())
        })
        .collect();

    let mut got = vec![0u64; MAX_SUBS];
    let mut buf = Vec::new();
    let mut now = 0;
    // 4 bars = 384 ticks; se cuentan solo los avisos de esos ticks.
    while clock.tick() < 384 {
        clock.advance(&at(now), 100_000);
        for (i, (_, sub)) in subs.iter().enumerate() {
            assert_eq!(sub.take(&mut buf), 0);
            got[i] += buf.drain(..).filter(|n| n.tick < 384).count() as u64;
        }
        now += 25_000;
    }
    for (i, (s, _)) in subs.iter().enumerate() {
        let expected = match *s {
            Subdivision::PerBar(n) => 4 * n as u64,
            Subdivision::EveryNBars(m) => 4u64.div_ceil(m as u64),
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
    let sub = clock.subscribe(Subdivision::PerBar(24)).unwrap();
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
    while clock.tick() < 192 {
        clock.advance(&at(now), 100_000);
        assert_eq!(tap.poll(&mut sink), 0);
        now += 25_000;
    }
    let marks: Vec<(u32, u8, u8)> = sink
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
