# Timeline view — Implementation plan

**Source spec:** `docs/product/timeline-view-design-spec.md` (2026-06-12) — **agreed baseline**, not a proposal. The spec inlines its prerequisites as P1–P3 (§4) and pins shared decisions in §5 "Settled behaviors" (trade-offs are §6); this plan carries the engineering detail for the same decisions.
**Scope:** web-first (core + wasm + vite). Firmware untouched; core API choices keep the ESP32 port viable.
**Author:** Technical Manager agent. Status: ready for execution. Alignment pass 2026-06-12: folded in the two post-review product decisions (euclid loop bracket locked; track-note control drum-lanes-only) and synced references to the renumbered spec.

---

## 1. What the spec requires, and what I verified against the code

### Requirements in one paragraph

A second, lossless lens over one canonical pattern model: a `Grilla | Timeline` view toggle; in timeline view every track is a lane on a **shared bar:beat ruler** (extent = longest loop, capped at 8 bars, no zoom/scroll in MVP); notes are blocks with tick-resolution start and duration; drum lanes (1 row) and pitch lanes (~2 octaves, octave buttons); per-track loop brackets with ghost repeats for visible polymeter; a playhead driven by the existing rAF loop; editing gestures (create/delete/move/resize/audition) with UI-side snap (`1/16 | 1/8 | 1/4 | off`); the **engine** enforces the same-pitch-overlap rule and all edits are legal during playback with no hung notes.

### Verified claims (spec ↔ code)

| Spec claim | Verified |
|---|---|
| `events_at` has a `tick % tps` early-return that must go | Yes — `pattern.rs:134` `if !tick.is_multiple_of(tps) { return; }` |
| `gate_ticks` is too narrow, widen to u32 | Yes — `Step.gate_ticks: u8`, `NoteEvent.gate_ticks: u8` (max 255 ticks ≈ 2.6 bars) |
| `pending_offs` guarantees offs survive edits | Yes — `PendingOff { due_tick, channel, note }` copies channel+note at fire time; deleting/moving a sounding note cannot hang it. `due_tick` is absolute (engine tick never wraps), so cross-loop-boundary offs are already safe |
| Grid is 5×16 toggle cells, drum-lane pitch model | Yes — `ui.ts`, `set_track_note` repaints all steps |
| rAF render loop exists | Yes — `draw()` in `main.ts`; `view()` is serialized via serde-wasm-bindgen **every frame** |
| Euclid E(k,n) ⇒ hits at lattice steps, track len = n | Yes — `apply_euclidean`, `len` |
| Playhead runs up to ~100 ms ahead of audible time | Yes — `advance()` consumes ticks to the horizon; `view()` reports `tick - 1` |

### Discrepancies and gaps the plan absorbs

1. ~~The referenced PRD does not exist.~~ **Resolved in the spec:** the earlier draft's PRD reference is gone; its prerequisite content is inlined as **P1** (canonical note model under the grid adapter), **P2** (note CRUD with stable IDs), **P3** (ViewModel + audition) in spec §4, and its settled points live in spec §5. This plan defines them concretely (Tasks 2–4); they remain the majority of the engineering work.
2. **There is no note model at all today.** `Track` is `Vec<Step>` + `len` in steps. "The model is canonical, the toggle is free" is the *target* state, not the current one. The migration is the riskiest task and goes first.
3. **No audition capability in core.** Needs design: note-offs for auditioned notes must fire even with transport stopped — today `pending_offs` only drains inside `on_tick`, which only runs while playing. Solved in §2.4.
4. **Grid UI is fixed at 16 cells** (`NUM_STEPS = 16` in `main.ts`; `Ui` builds 16 buttons). Loops up to 32 beats = 128 sixteenths cannot render fully in grid view. MVP caveat, now settled in spec §5: the grid shows the first 16 lattice steps with a hint; the timeline is the view for longer loops. Documented, not fixed.
5. **"Loop length in beats" vs euclid loops** — resolved and adopted into spec §3: core stores **arbitrary ticks** (`loop_ticks`, 1..=768); *beat-snapping is a UI affordance of the bracket drag only*, consistent with "snap never in core". (E(k,10) ⇒ `loop_ticks = 60` = 2.5 beats.)
6. **Overlap rule** — precisely defined in spec §5; engineering detail in §2.3 below.
7. **Lane-kind inference** — settled in spec §5: per-track `Option<LaneKind>` override; when `None`, infer `Pitch` iff the track has ≥2 distinct pitches, else `Drum`. Presentational only — playback never reads it (grep-level check in review).
8. Formerly open calls, **now settled in spec §5**: plain-click note length with snap *off* = 6 ticks (1/16); audition respects `channel_mode` (effective output channel), uses the note's velocity, length clamped to 1 beat so a 4-bar pad can't park a far-future note-off in the Web MIDI queue.
9. **New product decisions from PO review (this alignment pass):**
   - **Euclid loop bracket is locked.** On a euclidean track loop length is owned by n (`loop_ticks = n × 6`); a draggable bracket would desync them. Loop length changes via n in the gear. UI: no bracket drag on euclid lanes (Tasks 5, 8). Engine backstop: `set_loop_ticks` refuses on euclidean-mode tracks (§2.3) — my call, consistent with "rules enforced by the engine, not the UI".
   - **Track-note control hidden on pitch lanes.** The grid adapter keeps `set_track_note` = "repaint every note's pitch" (drum-lane semantics, §2.2), but surfacing it in a pitch lane's gear would silently flatten a melody with no undo in MVP. **UI-only rule:** the control renders on drum lanes only (Task 5; re-resolved per render, see Task 6).

---

## 2. Architecture decisions

### 2.1 The boundary, stated once

- **`obrero-core` owns:** the note model, note CRUD + overlap rule, loop length, audition emission, tick math, and the `ViewModel` (ticks only — no pixels, no snap, no selection). Both targets inherit identical editing semantics.
- **`obrero-wasm` owns:** thin bindings, the existing flat `[at_ms, status, d1, d2, len]` record format for anything that emits MIDI (audition reuses it).
- **`web/` owns:** view toggle, tick↔pixel geometry, canvas rendering, pointer/drag state machines, snap arithmetic, selection, lane octave window, and presentational visibility rules (e.g., which gear controls a lane kind shows). Anything that decides *what the data becomes* calls into core; anything that decides *what the screen shows* stays in TS. This is the structural mitigation for ESP32 drift the spec asks for (§6).

### 2.2 Core data model (replaces the Step grid)

```rust
// pattern.rs
pub type NoteId = u32;                       // stable per track, monotonic, never reused

pub struct Note {
    pub id: NoteId,
    pub start_tick: u32,                     // 0 <= start_tick (may exceed loop_ticks: dormant)
    pub len_ticks: u32,                      // >= 1
    pub pitch: u8,
    pub velocity: u8,
}

pub enum LaneKind { Drum, Pitch }            // presentational hint only

pub struct Track {
    pub channel: u8,
    pub notes: Vec<Note>,                    // sorted by start_tick (invariant)
    pub loop_ticks: u32,                     // 1..=32*PPQN; arbitrary ticks, NOT beat-quantized
    pub muted: bool,
    pub mode: StepMode,                      // Manual | Euclidean unchanged
    pub base_pitch: u8,                      // grid drum-lane pitch (replaces "note of all steps")
    pub lane_override: Option<LaneKind>,
    next_note_id: NoteId,
}
```

- `NoteEvent.gate_ticks` widens to `u32`. `PendingOff` unchanged.
- `events_at(tick)` loses the lattice early-return and becomes, per unmuted track: fire every note with `start_tick < loop_ticks && tick % loop_ticks == start_tick`. With sorted notes and ≤ a few hundred notes/track, a linear scan per tick is fine on both targets (ticks arrive at ≤ 48–120 Hz); no index structure until profiling says otherwise.
- `ticks_per_step` (6) stops being playback-relevant; it remains the grid view's lattice constant.

**The grid API survives as an adapter** — this is the load-bearing migration decision. `toggle_step`, `set_step`, `set_track_note`, `apply_euclidean`, `set_manual`, and the `StepView` grid in `ViewModel` are reimplemented *over notes*:

- Grid cell *i* is on ⇔ a note starts exactly at `i * 6`. Toggle on = `add_note(i*6, 3, base_pitch, 100)` (gate 3 preserves today's byte-exact output); toggle off = remove notes starting at `i*6`.
- `set_track_note(p)` sets `base_pitch` and rewrites every note's pitch (drum-lane semantics, preserves existing test). It stays in the engine with these semantics for both views; per spec §5 the **UI surfaces this control on drum lanes only** — on a pitch lane it would flatten a melody with no undo. UI rule, not an engine guard (the grid adapter and future automation legitimately call it).
- `apply_euclidean(k, n)` clears the track's notes, writes hits at `i*6` with `base_pitch`, sets `loop_ticks = n*6`. `set_manual` restores the pre-euclid `loop_ticks` (track stores it) so the existing "reopens full length" test stays green.
- Off-lattice notes are simply invisible/uneditable from the grid; data persists across toggles (lossless both ways, as specced).
- Shortening `loop_ticks` is non-destructive by construction: notes with `start_tick >= loop_ticks` stay in `Vec<Note>`, silent, rendered dimmed.

**Why core, not TS:** editing rules (overlap, clamping, ID allocation) must be identical for the future ESP32 editor, and core unit tests are the only cheap way to pin no-hung-note guarantees. **Why keep the adapter instead of porting the grid UI to note CRUD:** it keeps PR 2 free of any web change, lets the entire existing test suite act as a regression net for the migration, and the grid UI ships unmodified.

### 2.3 Note CRUD and the overlap rule (engine surface)

```rust
// engine.rs
pub fn add_note(&mut self, track: usize, start_tick: u32, len_ticks: u32,
                pitch: u8, velocity: u8) -> Option<NoteId>;
pub fn update_note(&mut self, track: usize, id: NoteId,
                   start_tick: u32, len_ticks: u32, pitch: u8) -> bool;
pub fn remove_note(&mut self, track: usize, id: NoteId) -> bool;
pub fn set_loop_ticks(&mut self, track: usize, ticks: u32) -> bool;   // clamps 1..=32*PPQN;
                                                                      // false (no-op) on euclidean tracks
pub fn set_track_lane(&mut self, track: usize, lane: Option<LaneKind>);
```

**Overlap rule (same pitch, same track, linear tick space within the loop)** — as settled in spec §5:

- A note's *start* may not fall inside another same-pitch note's span `[start, start+len)`, and two same-pitch notes may not share a start. `add_note`/`update_note` violating this → `None`/`false`; the UI snaps the drag back (spec's flash).
- A note's *duration* is **clipped** to the gap before the next same-pitch note's start. Create-drag and resize therefore always succeed once the start is legal — friendlier than rejecting, still one deterministic rule.
- Wrap-around overlap (a note sounding across the loop seam into a pitch-mate at tick 0) is **not policed in MVP**; the off still fires (absolute `due_tick`), worst case is an early cutoff, identical to today's grid behavior with long gates. Documented; a test pins the off-delivery.

**Euclid loop ownership (spec §2/§5, PO-settled):** on a euclidean track `loop_ticks = n × 6` is an invariant owned by the generator. `set_loop_ticks` returns `false` and changes nothing while `mode == Euclidean` — the engine backstop behind the UI's locked bracket, so no caller (web today, encoder UI later) can desync n from the loop. `apply_euclidean` itself sets `loop_ticks` internally, unaffected.

`update_note` is atomic move+resize+repitch (one call per drag frame is wasteful; the UI calls it on pointer-move *throttled to the drag's snapped deltas* and the engine just rewrites one struct — cheap). On `false` the UI keeps the last accepted geometry.

### 2.4 Audition (engine capability, transport-independent)

```rust
// engine.rs — sans-io, mirrors feed_midi_in's shape
pub fn audition(&mut self, track: usize, pitch: u8, velocity: u8,
                len_ticks: u32, now_us: u64, out: &mut Vec<TimedMidi>);
```

Emits a note-on at `now_us` on the track's *effective* channel (`channel_mode` applied) and registers the off in a new **time-based** pending list: `audition_offs: Vec<(due_us, channel, note)>`, drained at the **top of `advance()` before the transport/clock-source gates** (so it works stopped, and in External mode). `stop()`/`flush_pending_offs` also flushes them; `next_event_at()` returns the earliest audition off when sleeping (keeps the ESP32 contract honest). Length is converted with the current tick period and clamped to 1 beat (spec §5). The wasm wrapper returns the same flat records and `main.ts` pushes them through the existing `sendAll`.

Why not reuse `pending_offs`: those are tick-indexed and only drain while ticks run; audition must work with transport stopped. Two small lists with one flush path beats overloading one.

### 2.5 ViewModel additions (additive — grid fields stay)

```rust
pub struct NoteView { pub id: u32, pub start_tick: u32, pub len_ticks: u32,
                      pub pitch: u8, pub velocity: u8 }
// TrackView gains:
pub notes: Vec<NoteView>,    // ALL notes, incl. start_tick >= loop_ticks (drawn dormant)
pub loop_ticks: u32,
pub lane: LaneKind,          // resolved: override or inference
// ViewModel gains:
pub current_tick: u32,       // last emitted tick (tick-1 while playing), 0 stopped
```

The UI derives each lane's wrapped playhead (`current_tick % loop_ticks`), ghost-repeat count, and ruler extent from this — core stays pixel-free, and the same fields drive a future OLED strip renderer. The resolved `lane` field is also what drives the drum-lane-only visibility of the track-note gear control (Task 5).

**Per-frame serialization risk:** `view()` already crosses the wasm boundary at 60 Hz; adding note arrays grows it. Probe in Task 5 (see §4 R2); the prepared fallback is a cheap `current_tick() -> u32` wasm getter for the rAF path plus full `view()` only on an edit-dirty flag. Don't build the fallback until the probe fails.

### 2.6 Web layer structure

- `web/src/timeline.ts` — `TimelineView`: owns the timeline DOM (lane heads as DOM, **one `<canvas>` per lane** + one ruler canvas), draws from `ViewModel`, translates Pointer Events into engine calls. Canvases use `devicePixelRatio`-scaled backing stores; redraw notes layer on vm change, playhead every frame (single `clearRect` + lines — no dirty-rect machinery in MVP).
- `web/src/timeline-geometry.ts` — **pure functions**, no DOM: `tickToX/xToTick`, `pitchToY/yToPitch`, snap quantization, hit-testing (body vs. right-edge vs. bracket), drag-state reducers. This is the unit-testable nucleus of all gesture math.
- `web/src/track-head.ts` — track-head builder extracted from `ui.ts` (label, mute, channel, gear with note/euclid config + new lane-kind + octave controls) so grid and timeline share one implementation. Gear contents are conditional on the resolved lane kind: the **track-note control renders on drum lanes only** (spec §5); euclid config as today. (In grid view every row is drum-shaped, so the grid keeps the control unconditionally — no behavior change there.)
- `ui.ts` becomes the host: transport/device rows, the `Vista: Grilla | Timeline` toggle, and delegation of `render(vm)` to whichever view is active. Grid code is otherwise untouched.
- Pointer Events + `setPointerCapture` from day one; long-press (≈500 ms, <6 px movement) = delete on touch.
- View state (active view, snap, selection, drag, per-lane octave offset) lives in `TimelineView` instance fields. Nothing persists (persistence is explicitly post-MVP).

### 2.7 ESP32 considerations (kept open, not built)

- All new core APIs are no-std-friendly in spirit (no time sources, no I/O; `Vec` already used throughout and firmware uses std via ESP-IDF anyway).
- `next_event_at()` stays accurate (audition offs included) — the firmware's sleep contract survives.
- `LaneKind`, `NoteView`, `current_tick` are renderable on a 128×64 OLED; nothing assumes pointer input.
- Per the spec (§6): do **not** extend `input.rs`'s grid-shaped `Button::Step(n)` vocabulary for note editing; the timeline UI talks to the engine via the CRUD methods directly. The representation-agnostic input layer is deferred until hardware work resumes.

---

## 3. Work breakdown (ordered, PR-sized, each leaves `main` shippable)

> Each task ≈ one sitting. "Green" always means `cargo test -p obrero-core` plus a manual web smoke (`npm run wasm && npm run dev` in `web/`).

### Task 1 — Golden-output regression harness *(small PR, pure tests)*
**Goal:** pin today's emitted MIDI byte stream before touching the model.
**Files:** `crates/obrero-core/tests/golden.rs` (new).
**Work:** one test building a representative pattern through the *public* API (manual toggles incl. polymeter via euclid `n=12` against 16, a muted track, single-channel mode, a gate-255 hang + stop-flush, an external-clock replay) and asserting the full `TimedMidi` sequence over ≥2 loop cycles, both internal and external clock.
**Done when:** test passes against current `HEAD`. This is the migration safety net; it must merge *before* Task 2.

### Task 2 — Canonical note model under the existing API *(the migration, core only)*
**Goal:** spec prerequisite **P1**. `Track` becomes notes + `loop_ticks` (§2.2); every existing public engine method and `ViewModel` field behaves identically via the grid adapter.
**Files:** `pattern.rs` (rewrite), `engine.rs` (adapters, `gate_ticks → u32`), `view.rs` (derivation only — no new fields yet), `tests/engine.rs` (only if assertions touch removed internals — prefer zero edits).
**API changes:** none visible. `Pattern::starter()` now builds note-model tracks with `loop_ticks = 96`, `base_pitch = 60+i`.
**Tests:** entire existing suite + golden harness green, plus new: grid-toggle round-trip preserves an off-lattice note; `set_manual` restores pre-euclid `loop_ticks`; derived `StepView` matches note placement.
**Done when:** all green; web app manually indistinguishable. **No wasm/web changes in this PR** — that's the point of the adapter.
**Depends on:** Task 1.

### Task 3 — Note CRUD, stable IDs, overlap rule, loop length *(core only)*
**Goal:** spec prerequisite **P2**. §2.3 API verbatim.
**Files:** `engine.rs`, `pattern.rs` (sorted-insert helpers, overlap query), `lib.rs` re-exports, `tests/engine.rs` or new `tests/notes.rs`.
**Tests (each maps to a spec acceptance):**
- IDs stable across unrelated edits and never reused after delete.
- Overlap: start-inside-span rejected; duration clipped to next same-pitch start; different pitches coexist freely (spec §5).
- Off-lattice note (e.g. `start_tick = 26`) fires at the right µs at 120 BPM (T2 acceptance).
- Deleting a sounding note mid-gate: note-off still emitted (T2 acceptance).
- Note resized/placed across the loop boundary gets its off (T3 acceptance).
- `set_loop_ticks` shorter: out-of-loop notes silent but retained; restore length → they sound again (T4 acceptance).
- `set_loop_ticks` on a euclidean-mode track returns `false`, `loop_ticks` stays `n*6`; after `set_manual` it succeeds again (euclid bracket lock, spec §2/§5).
- 3-beat (`72`) vs 4-beat (`96`) tracks: note-on schedule realigns exactly every 288 ticks (T4 acceptance, polymeter).
**Done when:** green; still no web change.
**Depends on:** Task 2.

### Task 4 — Audition + ViewModel extension + wasm bindings *(core + wasm + TS types)*
**Goal:** spec prerequisite **P3** — everything the web UI will need, across the boundary.
**Files:** `engine.rs` (`audition`, `audition_offs`, `next_event_at`), `view.rs` (§2.5 fields), `crates/obrero-wasm/src/lib.rs` (bindings: `add_note → Option<u32>`, `update_note/remove_note → bool`, `set_loop_ticks → bool`, `set_track_lane(u8)`, `audition(...) -> Vec<f64>`, optional `current_tick() -> u32`), regenerate `web/src/pkg`, extend the `ViewModel`/`TrackView` interfaces in `web/src/ui.ts`.
**Tests:** audition emits on+off with transport stopped (off drained by a later `advance`); `stop()` flushes audition offs; `next_event_at` reports the audition off; external-clock mode audition works; view exposes `notes/loop_ticks/current_tick/lane` correctly (incl. lane inference and dormant notes).
**Done when:** green; grid UI still fully functional with regenerated pkg.
**Depends on:** Task 3.

### Task 5 — View toggle + read-only timeline *(web; spec story T1)*
**Goal:** switch views during playback and watch the pattern correctly.
**Files:** `web/src/timeline.ts`, `web/src/timeline-geometry.ts`, `web/src/track-head.ts` (extraction), `web/src/ui.ts` (toggle + delegation), `web/src/style.css`, `web/src/main.ts` (minor wiring).
**Work:** ruler (bars/beats, extent = max `loop_ticks` capped at 8 bars), drum + pitch lanes on canvas, dormant (out-of-loop) notes dimmed-hatched, loop bracket drawn (not draggable yet; on euclid lanes styled **locked** permanently, matching the lane's locked-blocks affordance), ghost repeats, euclid lanes with locked styling, playhead from `current_tick` each rAF, octave up/down on pitch lane heads. Track-head extraction makes gear contents lane-kind-aware: **track-note control on drum lanes only** (spec §5); grid view keeps it on all rows as today. **Run the serialization probe here** (§4 R2).
**Acceptance (spec T1):** instant lossless toggle both ways during playback; grid-built pattern rhythmically identical on the timeline; MIDI output identical in both views (by construction — verify via console record logging once); a pitch lane's gear shows no track-note control.
**Depends on:** Task 4.

### Task 6 — Create, delete, audition, snap *(web; spec story T2)*
**Goal:** put notes anywhere, hear them instantly.
**Files:** `timeline.ts`, `timeline-geometry.ts`, `style.css`; no core changes expected.
**Work:** snap selector (`1/16` default, `1/8`, `1/4`, `off`); click-drag create (click = one snap unit; snap off = 6 ticks, spec §5) via `add_note`; right-click / long-press delete via `remove_note`; audition on create and on click-select via `audition` records → `sendAll`; locked euclid lanes ignore edit gestures. Lane kind is re-resolved from each `ViewModel` (nothing cached): adding a second distinct pitch to a drum lane flips it to a pitch lane on the next render, and the gear's track-note control disappears with it (spec §5 — inference + drum-lane-only control).
**Acceptance (spec T2):** with snap off, a note placed 2 ticks after a beat audibly plays offset; deleting a sounding note never hangs (core-tested; manual spot check); a lane that gains a second pitch re-renders as a pitch lane with the track-note control hidden.
**Depends on:** Task 5.

### Task 7 — Move and resize with snap-back *(web; spec story T3)*
**Goal:** refine timing/pitch/duration while the loop plays.
**Files:** `timeline.ts`, `timeline-geometry.ts`, `style.css`.
**Work:** drag state machine (body = move time+pitch, right edge = resize, min 1 tick); per-move `update_note`; on `false`, revert to last accepted geometry + brief flash; pointer capture; touch parity.
**Acceptance (spec T3):** edits audible on the next pass with transport running; rejected drags snap back; no hung notes.
**Depends on:** Task 6.

### Task 8 — Loop bracket drag *(web; spec story T4)*
**Goal:** 3-against-4 polymeter by gesture.
**Files:** `timeline.ts`, `timeline-geometry.ts`; core already done (Task 3).
**Work:** draggable bracket, beat-snapped *in the UI*, 1–32 beats; ghost repeats and ruler extent update live; dormant notes restyle immediately; grid view reflects shorter loops (cells beyond hidden — existing behavior). **Euclid lanes are excluded from bracket drag** (spec §2/§5: loop length is owned by n; change it via n in the gear): hit-testing never returns the bracket zone on a euclid lane, the bracket keeps its locked styling from Task 5, and the engine's `set_loop_ticks` euclid guard (Task 3) backstops any path that slips through.
**Acceptance (spec T4):** 3-beat bass against 4-beat drums audibly phases and visibly realigns every 3 bars on the ruler; dragging on a euclid lane's bracket does nothing (no drag affordance, no engine call); switching that track to manual makes the bracket draggable.
**Depends on:** Task 7 (only for the shared drag plumbing; could swap with 7 if needed).

**Post-MVP queue (acknowledged, not planned):** undo, persistence (first two, per spec §4), playhead lookahead correction (UI-side, using emitted-record timestamps; spec §5 ships the artifact as-is), velocity editing, zoom/scroll, secondary per-lane playhead ticks.

---

## 4. Risk assessment

| # | Risk | Likelihood/Impact | Mitigation |
|---|---|---|---|
| R1 | **Model migration regresses playback semantics** (grid, euclid, polymeter, external clock, channel modes) | Med / High | Golden harness (Task 1) merged *before* the rewrite; grid adapter keeps the whole existing suite as a net; zero web changes in the migration PR isolates blame. |
| R2 | **`view()` serialization per rAF frame too heavy** with note arrays (serde-wasm-bindgen at 60 Hz) | Low–Med / Med | Probe in Task 5: 5 tracks × 200 notes, measure `view()` ms in DevTools. Budget: <1 ms. Fallback (designed, not built): `current_tick()` scalar getter for the per-frame path + full `view()` only after edits. |
| R3 | **Audition offs lost when transport stopped** → hung notes from the editor | Med / High | Designed out: time-based `audition_offs` drained at top of `advance()` before all gates; flushed on stop; unit-tested in Task 4. Length clamp (1 beat, spec §5) bounds Web MIDI's unsendable queued offs if the user switches output mid-audition. |
| R4 | **Drag/hit-test math bugs** (DPR scaling, snap rounding, edge vs. body zones, touch) | Med / Med | All geometry in pure `timeline-geometry.ts`; vitest covers it (§5); manual matrix incl. one tablet/touch pass per web task. |
| R5 | **Overlap-rule edge cases** (clip-to-zero, wrap seam, equal starts) | Med / Low–Med | Rule pinned exactly in §2.3 (= spec §5) with table-driven core tests; wrap seam explicitly out of MVP scope with a test documenting the off still fires. |
| R6 | **Adapter ambiguity: grid edits vs. off-lattice notes** (toggle near, not on, an off-lattice note) | Low / Low | Rule pinned: grid cell binds only to `start_tick == i*6` exactly; round-trip test in Task 2. |
| R7 | **Grid can't show loops >16 steps** (fixed 16 buttons) | Certain / Low | Accepted MVP caveat (spec §5; item 4 in §1 above); status hint in grid view when a track's loop exceeds 16 lattice steps. |
| R8 | **ESP32 door:** `next_event_at` contract, no-pixels rule, input vocabulary creep | Low / Med later | Audition offs included in `next_event_at` (tested); lane kind is view-only (review grep: playback path never reads `lane_override`); no `input.rs` extensions in any task. |
| R9 | **Playhead visibly ahead of audio** (~100 ms lookahead) | Certain / Low | Ship as-is per spec §5; correction is a queued post-MVP UI story. Don't attempt in core. |
| R10 | **Euclid desync paths** (`loop_ticks` drifting from n×6 via some future caller) | Low / Med | Engine guard: `set_loop_ticks` is a no-op on euclidean tracks (Task 3, tested) — the invariant holds regardless of UI discipline, web or hardware. |

The ordering puts R1 (Tasks 1–2) and R3 (Task 4) — the two ways this project gets *stuck* — before any pixels are drawn.

---

## 5. Test strategy

### Core unit tests (`crates/obrero-core/tests/`) — the contract layer
- **Golden harness (Task 1):** full emitted-bytes equivalence across the migration; kept forever as the playback-semantics pin.
- **Model invariants (Tasks 2–3):** sorted notes, ID stability/no-reuse, overlap accept/reject/clip table, loop-shorten non-destructiveness, grid↔notes adapter round-trips, `set_manual` loop restore, `set_loop_ticks` euclid no-op.
- **Timing & note-off guarantees (Tasks 3–4):** off-lattice µs placement; delete-while-sounding off; cross-loop-boundary off; 3v4 realignment at tick 288; audition on/off while stopped, under external clock, and flushed by stop; `next_event_at` with audition pending.
- Every spec acceptance criterion from stories T1–T4 that is *audible* maps to one of the above; the table in each task lists the mapping.

### Web layer
- **Add vitest** (one devDependency, zero config with vite) scoped to `timeline-geometry.ts` only: tick↔px round-trips at several widths/DPRs, snap quantization incl. `off`, hit-test zone classification (incl. "bracket zone never returned on euclid lanes"), drag reducers. No DOM/canvas testing — not worth the harness cost for a solo project.
- TypeScript strictness (`tsc` already in `npm run build`) guards the ViewModel interface against wasm regeneration drift.

### Manual verification checklist (per web task, in Chrome + one touch device)
1. Build a pattern in grid → toggle to timeline → rhythm visually identical; toggle back lossless (T1).
2. With a MIDI monitor (or console-log of `sendAll` records): identical records in both views for the same pattern (T1).
3. Snap off: place a note 2 ticks after beat 2; hear the push (T2).
4. Delete and resize notes *while playing*; listen for hung notes across ≥4 loop passes (T2/T3).
5. Drag a note onto a same-pitch neighbor: snap-back + flash (T3).
6. Bracket: bass to 3 beats over 4-beat drums; confirm audible phasing and on-ruler realignment every 3 bars; restore to 4 beats and confirm dormant notes return (T4).
7. External clock from another device: playhead follows, all edits remain live (regression).
8. Euclid lane: blocks locked; bracket locked — dragging it does nothing; E(k,n) edit via gear reflows the lane *and* its loop bracket; switching to manual unlocks both (T4/regression).
9. Gear menu: track-note control present on a drum lane, absent on a pitch lane; give a drum lane a second pitch and confirm the control disappears on re-render (T1/T2).

---

## Appendix — decision log (spec open points, since adopted into spec §5)

| Point | Decision | Status |
|---|---|---|
| Overlap "rejected/clipped" | Start-collisions rejected; durations clipped to next same-pitch start | Settled — spec §5 (revisit clipping if it feels wrong in play-testing) |
| Loop length "in beats" | Core stores arbitrary ticks (euclid needs it); beat snap is bracket-UI only | Settled — spec §3 |
| Lane kind inference | `Option<LaneKind>` override; else ≥2 pitches ⇒ Pitch | Settled — spec §5 |
| Default click-create length, snap off | 6 ticks | Settled — spec §5 |
| Audition channel/velocity/length | Effective channel via `channel_mode`; note velocity; length clamped to 1 beat | Settled — spec §5 |
| Grid view of >16-step loops | First 16 cells + hint; timeline is the long-loop view | Settled — spec §5 (MVP caveat) |
| Note CRUD vs grid adapter coexistence | Adapter kept indefinitely (grid stays grid-shaped) | Settled — this plan §2.2 |
| Euclid loop bracket | Locked in UI (no drag on euclid lanes); engine `set_loop_ticks` no-op backstop while Euclidean | Settled — spec §2/§5 (PO review); guard is a TM call |
| Track-note control on pitch lanes | Engine semantics unchanged (repaint all pitches); UI shows the control on drum lanes only | Settled — spec §5 (PO review) |
