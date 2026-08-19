use obrero_core::engine::{Engine, TimedMidi, MIDI_CLOCK, MIDI_START, MIDI_STOP};
use obrero_core::{ClockSource, InputEvent};

fn clocks(events: &[TimedMidi]) -> Vec<u64> {
    events
        .iter()
        .filter(|e| e.bytes[0] == MIDI_CLOCK)
        .map(|e| e.at_us)
        .collect()
}

fn notes(events: &[TimedMidi]) -> Vec<(u64, u8, u8, u8)> {
    events
        .iter()
        .filter(|e| e.bytes[0] & 0xF0 == 0x90 || e.bytes[0] & 0xF0 == 0x80)
        .map(|e| (e.at_us, e.bytes[0], e.bytes[1], e.bytes[2]))
        .collect()
}

#[test]
fn internal_clock_24ppqn_at_120bpm() {
    // 120 BPM = 2 negras/s × 24 PPQN = 48 clocks por segundo
    let mut e = Engine::new();
    let mut out = Vec::new();
    e.play();
    e.advance(0, 999_999, &mut out);

    assert_eq!(out[0].bytes[0], MIDI_START);
    let ticks = clocks(&out);
    assert_eq!(ticks.len(), 48);
    assert_eq!(ticks[0], 0);
    // tick 47 a 47 × 20833.33 µs
    assert!((ticks[47] as i64 - 979_166).abs() <= 1);
}

#[test]
fn no_drift_over_simulated_minutes() {
    // 5 minutos a 120 BPM avanzando en ventanas de 25 ms (como la web)
    let mut e = Engine::new();
    let mut out = Vec::new();
    e.play();
    let mut all = Vec::new();
    let mut now: u64 = 0;
    while now < 300_000_000 {
        e.advance(now, 100_000, &mut out);
        all.append(&mut out);
        now += 25_000;
    }
    let ticks = clocks(&all);
    // El último tick emitido debe seguir alineado a n × 20833.333… µs
    let last = *ticks.last().unwrap();
    let n = ticks.len() as f64 - 1.0;
    let expected = n * (60_000_000.0 / (120.0 * 24.0));
    assert!(
        (last as f64 - expected).abs() < 2.0,
        "drift: last={last} expected={expected}"
    );
}

#[test]
fn overlapping_advance_windows_never_double_emit() {
    let mut e = Engine::new();
    let mut all = Vec::new();
    let mut out = Vec::new();
    e.play();
    // Ventanas muy solapadas: lookahead 100 ms, paso 10 ms
    for i in 0..100u64 {
        e.advance(i * 10_000, 100_000, &mut out);
        all.append(&mut out);
    }
    let ticks = clocks(&all);
    for pair in ticks.windows(2) {
        assert!(
            pair[0] < pair[1],
            "timestamps duplicados o fuera de orden: {pair:?}"
        );
    }
}

#[test]
fn step_fires_note_on_and_gated_note_off() {
    let mut e = Engine::new();
    e.toggle_step(0, 0); // track 0: nota 60, canal 0, gate 3 ticks
    let mut out = Vec::new();
    e.play();
    e.advance(0, 999_999, &mut out);

    let ns = notes(&out);
    assert_eq!(ns.len(), 2);
    let (t_on, st_on, note, vel) = ns[0];
    assert_eq!((st_on, note, vel), (0x90, 60, 100));
    assert_eq!(t_on, 0);
    let (t_off, st_off, note_off, _) = ns[1];
    assert_eq!((st_off, note_off), (0x80, 60));
    // gate de 3 ticks = 62500 µs a 120 BPM
    assert!((t_off as i64 - 62_500).abs() <= 1);
}

#[test]
fn pattern_loops_every_16_steps() {
    let mut e = Engine::new();
    e.toggle_step(0, 0);
    let mut out = Vec::new();
    e.play();
    // 16 pasos × 6 ticks × 20833.33 µs = 2 s por vuelta; avanzar 2 vueltas
    e.advance(0, 3_999_999, &mut out);
    let ons: Vec<_> = notes(&out)
        .into_iter()
        .filter(|(_, s, _, _)| *s == 0x90)
        .collect();
    assert_eq!(ons.len(), 2);
    assert!((ons[1].0 as i64 - 2_000_000).abs() <= 1);
}

#[test]
fn stop_lets_current_step_finish_its_gate_before_cutting() {
    let mut e = Engine::new();
    // gate corto (3 ticks); track 1 también dispara en el paso 1 para
    // comprobar que, tras el Stop, ese próximo paso NO suena.
    e.toggle_step(0, 0);
    e.toggle_step(0, 1);
    let mut out = Vec::new();
    e.play();
    e.advance(0, 0, &mut out); // dispara el paso 0 (nota 60, gate 3 ticks)
    assert!(notes(&out).iter().any(|(_, s, _, _)| *s == 0x90));

    out.clear();
    e.stop(); // primer toque: no corta ya, deja terminar el paso en curso
    e.advance(20_000, 100_000, &mut out);
    // El corte llega recién cuando se apaga la nota colgada (gate 3 ticks
    // ≈ 62500 µs), no antes.
    assert!(out.iter().any(|ev| ev.bytes[0] == MIDI_STOP));
    let offs: Vec<_> = notes(&out)
        .into_iter()
        .filter(|(_, s, _, _)| *s == 0x80)
        .collect();
    assert_eq!(offs.len(), 1);
    assert_eq!(offs[0].2, 60);
    // El paso 1 (tick siguiente) nunca dispara: el Stop lo previno.
    assert!(!out.iter().any(|ev| ev.bytes[0] == 0x90));
    assert!(!e.view().playing);
}

#[test]
fn double_stop_forces_immediate_silence() {
    let mut e = Engine::new();
    // gate larguísimo para que la nota quede colgada
    e.pattern_mut().tracks[0].steps[0].on = true;
    e.pattern_mut().tracks[0].steps[0].gate_ticks = 255;
    let mut out = Vec::new();
    e.play();
    e.advance(0, 50_000, &mut out);
    assert!(notes(&out).iter().any(|(_, s, _, _)| *s == 0x90));

    out.clear();
    e.stop(); // primer toque: entra en modo "apagando"
    assert!(e.view().playing); // sigue sonando mientras termina el paso
    e.stop(); // segundo toque: silencio general ya
    e.advance(60_000, 0, &mut out);
    assert_eq!(out[0].bytes[0], MIDI_STOP);
    assert_eq!(out[0].at_us, 60_000);
    let offs: Vec<_> = notes(&out)
        .into_iter()
        .filter(|(_, s, _, _)| *s == 0x80)
        .collect();
    assert_eq!(offs.len(), 1);
    assert_eq!(offs[0].2, 60);
    assert!(!e.view().playing);
}

#[test]
fn external_clock_follow_matches_internal_output() {
    // Patrón: track 0 en pasos 0 y 8
    let make = || {
        let mut e = Engine::new();
        e.toggle_step(0, 0);
        e.toggle_step(0, 8);
        e
    };

    // Interno: 2 segundos = una vuelta completa
    let mut internal = make();
    let mut out_i = Vec::new();
    internal.play();
    internal.advance(0, 1_999_999, &mut out_i);
    let notes_i: Vec<_> = notes(&out_i)
        .iter()
        .map(|(_, s, n, v)| (*s, *n, *v))
        .collect();

    // Externo: Start + 96 clocks (16 pasos × 6 ticks)
    let mut external = make();
    external.set_clock_source(ClockSource::External);
    let mut out_e = Vec::new();
    external.feed_midi_in(&[0xFA], 0, &mut out_e);
    for i in 0..96u64 {
        external.feed_midi_in(&[0xF8], i * 20_833, &mut out_e);
    }
    let notes_e: Vec<_> = notes(&out_e)
        .iter()
        .map(|(_, s, n, v)| (*s, *n, *v))
        .collect();

    assert_eq!(notes_i, notes_e);
    assert!(!notes_e.is_empty());
    // Como esclavo no re-emite clock ni Start
    assert!(clocks(&out_e).is_empty());
    assert!(!out_e.iter().any(|e| e.bytes[0] == MIDI_START));
}

#[test]
fn external_stop_flushes_hanging_notes() {
    let mut e = Engine::new();
    e.pattern_mut().tracks[0].steps[0].on = true;
    e.pattern_mut().tracks[0].steps[0].gate_ticks = 255;
    e.set_clock_source(ClockSource::External);
    let mut out = Vec::new();
    e.feed_midi_in(&[0xFA, 0xF8], 0, &mut out);
    assert!(notes(&out).iter().any(|(_, s, _, _)| *s == 0x90));
    out.clear();
    e.feed_midi_in(&[0xFC], 1_000, &mut out);
    let offs: Vec<_> = notes(&out)
        .into_iter()
        .filter(|(_, s, _, _)| *s == 0x80)
        .collect();
    assert_eq!(offs.len(), 1);
}

#[test]
fn tempo_change_applies_to_future_ticks() {
    let mut e = Engine::new();
    let mut out = Vec::new();
    e.play();
    e.advance(0, 0, &mut out); // ancla + tick 0
    e.set_tempo(60.0); // período pasa de 20833 a 41667 µs
    out.clear();
    e.advance(0, 99_999, &mut out);
    let ticks = clocks(&out);
    // ticks a ~20833 (agendado antes del cambio), luego cada 41667
    assert!((ticks[0] as i64 - 20_833).abs() <= 1);
    assert!((ticks[1] as i64 - 62_500).abs() <= 1);
}

#[test]
fn input_events_drive_transport_and_tempo() {
    use obrero_core::Button;
    let mut e = Engine::new();
    e.handle_input(InputEvent::EncoderDelta(10));
    assert_eq!(e.bpm(), 130.0);
    e.handle_input(InputEvent::ButtonDown(Button::Play));
    assert!(e.view().playing);
    e.handle_input(InputEvent::ButtonDown(Button::Step(3)));
    assert!(e.view().tracks[0].steps[3].on);
    e.handle_input(InputEvent::ButtonDown(Button::Stop));
    assert!(!e.view().playing);
}

#[test]
fn track_channel_change_applies_to_emitted_notes() {
    let mut e = Engine::new();
    e.toggle_step(0, 0);
    e.set_track_channel(0, 4);
    let mut out = Vec::new();
    e.play();
    e.advance(0, 999_999, &mut out);
    let ns = notes(&out);
    // Note On y Note Off salen por el canal nuevo (0x94/0x84 = canal 5 humano)
    assert_eq!(ns[0].1, 0x94);
    assert_eq!(ns[1].1, 0x84);
    // El canal se enmascara a 0-15
    e.set_track_channel(0, 200);
    assert_eq!(e.view().tracks[0].channel, 200 & 0x0F);
}

#[test]
fn single_channel_mode_routes_all_tracks_through_one_channel() {
    use obrero_core::ChannelMode;
    let mut e = Engine::new();
    e.toggle_step(0, 0); // track 0, nota 60
    e.toggle_step(1, 0); // track 1, nota 61
    e.set_track_channel(0, 2);
    e.set_track_channel(1, 5);
    e.set_channel_mode(ChannelMode::Single(7));
    let mut out = Vec::new();
    e.play();
    e.advance(0, 50_000, &mut out);
    let ons: Vec<_> = notes(&out)
        .into_iter()
        .filter(|(_, s, _, _)| *s & 0xF0 == 0x90)
        .collect();
    // Ambos tracks salen por el canal 7 (0x97), distinguidos por nota
    assert_eq!(ons.len(), 2);
    assert!(ons.iter().all(|(_, s, _, _)| *s == 0x97));
    let mut played: Vec<u8> = ons.iter().map(|(_, _, n, _)| *n).collect();
    played.sort();
    assert_eq!(played, vec![60, 61]);

    // Volver a PerTrack restaura los canales propios
    e.stop();
    let mut out2 = Vec::new();
    e.advance(60_000, 0, &mut out2); // flush del stop
    e.set_channel_mode(ChannelMode::PerTrack);
    out2.clear();
    e.play();
    e.advance(100_000, 50_000, &mut out2);
    let mut statuses: Vec<u8> = notes(&out2)
        .into_iter()
        .filter(|(_, s, _, _)| *s & 0xF0 == 0x90)
        .map(|(_, s, _, _)| s)
        .collect();
    statuses.sort();
    assert_eq!(statuses, vec![0x92, 0x95]);
}

#[test]
fn set_track_note_changes_emitted_note() {
    let mut e = Engine::new();
    e.toggle_step(0, 0);
    e.set_track_note(0, 49); // retonificar el lane completo
    let mut out = Vec::new();
    e.play();
    e.advance(0, 50_000, &mut out);
    let ns = notes(&out);
    assert_eq!(ns[0].2, 49);
    assert_eq!(e.view().tracks[0].note, 49);
    // La nota se enmascara a 0-127
    e.set_track_note(0, 200);
    assert_eq!(e.view().tracks[0].note, 200 & 0x7F);
}

#[test]
fn euclidean_generates_classic_patterns() {
    let mut e = Engine::new();
    // E(3,8) = tresillo: golpes en 0, 3, 6
    e.set_track_euclidean(0, 3, 8);
    let vm = e.view();
    assert_eq!(vm.tracks[0].steps.len(), 8);
    let hits: Vec<usize> = vm.tracks[0]
        .steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.on)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(hits, vec![0, 3, 6]);
    let params = vm.tracks[0].euclidean.unwrap();
    assert_eq!((params.pulses, params.steps), (3, 8));

    // E(5,8) = x.x.xx.x
    e.set_track_euclidean(0, 5, 8);
    let hits: Vec<usize> = e.view().tracks[0]
        .steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.on)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(hits, vec![0, 2, 4, 5, 7]);
}

#[test]
fn euclidean_track_plays_and_loops_at_its_own_length() {
    let mut e = Engine::new();
    e.set_track_euclidean(0, 3, 8);
    let mut out = Vec::new();
    e.play();
    // 8 pasos × 6 ticks × 20833.33 µs = 1 s por vuelta a 120 BPM; 2 vueltas
    e.advance(0, 1_999_999, &mut out);
    let ons: Vec<_> = notes(&out)
        .into_iter()
        .filter(|(_, s, _, _)| *s == 0x90)
        .collect();
    assert_eq!(ons.len(), 6); // 3 golpes por vuelta × 2
}

#[test]
fn euclidean_mode_ignores_manual_toggles_until_back_to_manual() {
    let mut e = Engine::new();
    e.set_track_euclidean(0, 3, 8);
    e.toggle_step(0, 1); // ignorado: la grilla la define E(k,n)
    assert!(!e.view().tracks[0].steps[1].on);

    e.set_track_manual(0);
    assert!(e.view().tracks[0].euclidean.is_none());
    assert_eq!(e.view().tracks[0].steps.len(), 16); // largo completo de vuelta
    e.toggle_step(0, 1);
    assert!(e.view().tracks[0].steps[1].on);
    // El patrón generado se conserva como punto de partida manual
    assert!(e.view().tracks[0].steps[0].on);
}

#[test]
fn euclidean_params_are_clamped() {
    let mut e = Engine::new();
    // pulses > steps se recorta a steps; steps 0 se eleva a 1
    e.set_track_euclidean(0, 9, 4);
    let params = e.view().tracks[0].euclidean.unwrap();
    assert_eq!((params.pulses, params.steps), (4, 4));
    e.set_track_euclidean(0, 1, 0);
    assert_eq!(e.view().tracks[0].euclidean.unwrap().steps, 1);
}

#[test]
fn muted_track_emits_nothing_until_unmuted() {
    let mut e = Engine::new();
    e.toggle_step(0, 0);
    e.toggle_track_mute(0);
    assert!(e.view().tracks[0].muted);
    let mut out = Vec::new();
    e.play();
    e.advance(0, 999_999, &mut out);
    assert!(notes(&out).is_empty());

    e.toggle_track_mute(0);
    assert!(!e.view().tracks[0].muted);
    out.clear();
    // Siguiente vuelta del loop: paso 0 vuelve a caer en t = 2 s (120 BPM)
    e.advance(1_000_000, 1_000_000, &mut out);
    assert!(notes(&out).iter().any(|(_, s, _, _)| *s == 0x90));
}

#[test]
fn next_event_at_reports_upcoming_tick() {
    let mut e = Engine::new();
    assert_eq!(e.next_event_at(), None);
    e.play();
    assert_eq!(e.next_event_at(), Some(0)); // start pendiente
    let mut out = Vec::new();
    e.advance(1_000, 0, &mut out);
    let next = e.next_event_at().unwrap();
    assert!((next as i64 - 21_833).abs() <= 1); // 1000 + 20833
}
