# Timeline view — Product/UX design spec

**obrero-1, web-first.** Author: PO agent. Status: agreed baseline, consistent with `docs/engineering/timeline-view-implementation-plan.md` (2026-06-12), which carries the engineering detail for the same decisions. An earlier draft referenced a separate PRD (`docs/product/timeline-style-prd.md`); that document was never committed — its prerequisite content is inlined in §4 and its settled points in §5.

---

## 1. Concept

### What it is

A pattern-level view where **horizontal distance is musical time** (in ticks, against a bar:beat ruler shared by all tracks), each track is a horizontal **lane**, and each event is a **block with a start and a duration**. A vertical playhead sweeps across all lanes in sync with the transport.

This is the second lens over the canonical note model defined as a prerequisite in §4 (`Note { id, start_tick, len_ticks, pitch, velocity }` per track, per-track `loop_ticks`). The grid stays; nothing about the grid workflow changes.

### What it solves that the grid cannot

The grid (`web/src/ui.ts`: 5 tracks × 16 toggle cells) answers "*which* semiquaver slots fire?". It structurally cannot express:

1. **Duration as a gesture.** `gate_ticks` exists in core but the grid UI doesn't even expose it. Legato bass vs. staccato stabs is *the* difference between two takes of the same pattern; on a timeline it's dragging a note's right edge.
2. **Off-lattice timing.** The engine resolves 24 PPQN — 6 positions inside every grid cell that today are unreachable. Pushed hats, anticipated chord changes, humanized timing.
3. **Melodic phrasing.** `set_track_note` paints one pitch across a whole track (drum-lane model). A melody needs per-event pitch — a compact piano-roll lane.
4. **Polymeter you can *see*.** Per-track `len` already lets a 12-step track phase against 16. But the grid hides the relationship — both rows just look like rows. On a shared ruler, a 3-beat loop visibly drifts against a 4-beat loop, with ghost repeats showing exactly where they realign. This is the timeline's signature demo and why the shared ruler is non-negotiable.
5. **Phrases longer than 16 steps** without rendering 64+ tiny cells.

### Rejected alternative: per-lane timeline styling

An earlier draft proposed a per-track style picker where a timeline lane replaces a grid row inside the grid page, each lane's loop scaled to fit its row width. Rejected: **per-lane time scaling destroys the time axis** — a 1-bar drum lane and a 4-bar melody lane would be the same pixel width at different time scales, which makes "timeline" a lie and polymeter illegible. Instead: **one view toggle (`Grilla | Timeline`) at pattern level; in timeline view, every track renders as a lane against one shared ruler.** Because the model is canonical, the toggle is free and lossless in both directions — no conversion ceremony, no quantize-on-switch; you only quantize if you *want* to, as an explicit action later. The earlier draft's other foundations (model unification, engine-enforced overlap rule, audition as an engine capability, snap as a UI-only edit aid) stand, specced in §4 and §5.

---

## 2. Layout & interaction design

### Page layout

Transport and device rows are unchanged (`ui.ts` top blocks). Below them, a view toggle and the timeline surface:

```
┌──────────────────────────────────────────────────────────────────────┐
│ ▶ Play  ■ Stop   BPM [120]  Reloj [Interno ▾]                        │
│ Salida MIDI [...]  Entrada MIDI [...]  Canales [...]                 │
│                                            Vista: [ Grilla │▐Timeline▌] │
├──────────────┬───────────────────────────────────────────────────────┤
│              │ 1       ·       2       ·       3       ·       4     │ ← ruler (bars,
│              │ │   ▼ playhead                                        │   beat ticks)
├──────────────┼─┼─────────────────────────────────────────────────────┤
│ Kick    M Ch1│█│██   ▒██   ███    ██  ║░░░░ ░░   ░░░    ░░ ░║        │ ← drum lane
│         ⚙   │ │                      ↑loop end (1 bar)  ghosts→      │   (1 row)
├──────────────┼─┼─────────────────────────────────────────────────────┤
│ Hats    M Ch1│█│█ █ █ █ █ █ █ █ █ █ █ █ █ █║░ ░ ░ ░ ░ ░ ░ ░ ░ ░║    │ ← euclid track:
│      E(7,16)⚙│ │                                                     │   same lane, locked
├──────────────┼─┼─────────────────────────────────────────────────────┤
│ Bass    M Ch2│ │  C3 ▁▁▁                       ▁▁▁▁▁▁                │ ← pitch lane
│   [▴ oct ▾] ⚙│█│      ▔▔▔ G2▆▆▆▆▆▆▆     E2▆▆▆        F2 ▆▆▆▆▆▆▆▆▆▆▆▆│   (~2 octaves,
│              │ │  snap [1/16 ▾]                            (4 bars)  │   taller row)
├──────────────┼─┴─────────────────────────────────────────────────────┤
│ + lane head column: same controls as today's track-head (label,     │
│   mute, channel, gear→ note/euclid config)                          │
└──────────────────────────────────────────────────────────────────────┘
```

- **Ruler**: bar numbers, beat subdivisions; one ruler for everything. View extent = the longest track loop (MVP: capped at 8 bars).
- **Lane kinds** (presentation only, inferred + user-overridable in the gear menu):
  - **Drum lane** — 1 row tall, all notes share the track pitch (today's model). Blocks still show duration.
  - **Pitch lane** — taller; vertical axis is pitch, ~2 octaves visible with octave up/down buttons in the lane head. Fixed window, no vertical zoom in MVP (deliberately the simpler option).
- **Loop region per track**: a bracket `║` at `loop_ticks`, draggable along the lane's top edge, snapping to beats. Beyond it, **ghost repeats** (the loop's notes redrawn dimmed) up to the view extent, so phasing against longer tracks is visible. Dragging the bracket left is non-destructive: notes past the new end stay in data, drawn dimmed-and-hatched, silent — exactly how grid `len` behaves today.
- **Playhead**: one vertical line through ruler and all lanes. Each track also gets a secondary tick (▼) at its own wrapped position inside ghost regions — optional polish, not MVP.

### Editing gestures

| Gesture | Action |
|---|---|
| Click-drag on empty lane | Create note: press sets start + pitch, drag right sets duration. Plain click = default length (one snap unit). |
| Drag note body | Move in time and (pitch lanes) pitch. Snap-aware. |
| Drag note right edge | Resize duration. Min 1 tick. |
| Right-click / long-press (touch) | Delete. |
| Snap selector (per view, in the toolbar): `1/16` (default) / `1/8` / `1/4` / `off` | Edit-time quantize only. Never stored, never in core. |
| Drag loop bracket | Set `loop_ticks`. Beat-snap applied in the UI only; core stores ticks (§3). Locked on euclid tracks. |
| Click a note | Select + **audition**: immediate note-on/scheduled off to the current MIDI output, transport-independent. Creating a note auditions too. |

Rules enforced by the **engine**, not the UI (one rule for both targets): the same-pitch overlap rule, defined precisely in §5; rejected drags snap back with a brief flash. All edits are legal during playback — the engine reads the pattern per tick and `pending_offs` already guarantees sounding notes get their offs (`engine.rs`), so live tweaking never stops the transport. That immediacy is an acceptance criterion, not a nice-to-have.

Euclidean tracks render in the timeline like any other lane (the generator writes lattice notes into the canonical model), but their blocks are **locked** (same `locked` affordance the grid uses) — edit E(k,n) in the gear, or switch the track to manual to take ownership of the notes. The loop bracket is locked too: on a euclidean track loop length is owned by n (`loop_ticks = n × 6`); change it by changing n.

### Zoom & scroll

**MVP: none.** The view fits the longest loop; at 8 bars on a desktop width that's ~1px/tick, enough to grab a 6-tick note. Post-MVP: stepped zoom (fit / 2 bars / 1 bar) + horizontal scroll with the playhead optionally following. Don't build scroll until someone makes a pattern long enough to need it.

---

## 3. Transport & time model

- **Ticks are the only time currency.** 24 PPQN (`pattern.rs::PPQN`), quarter = 24 ticks, 4/4 bar = 96. The UI maps ticks→pixels; core never sees pixels. "Free placement" honestly means *free at 24 PPQN* (~20.8 ms at 120 BPM); PPQN bumps stay deferred until a musician feels the ceiling.
- **Tempo** is untouched: BPM scales tick period in `advance()`; timeline geometry is tempo-independent. External clock mode works identically — the playhead advances on incoming `0xF8`, the view doesn't know or care.
- **No time-signature system.** Core stores loop length **in ticks** (`loop_ticks`, 1..=768, i.e. up to 8 bars) — not beats, because euclidean loops are already non-integer beat counts (E(k,10) ⇒ 60 ticks = 2.5 beats). **Beat-snapping is a UI affordance of the bracket drag only** (1–32 beats), consistent with "snap is never stored, never in core". The ruler assumes groups of 4 beats for bar numbering. A 7-beat loop *is* 7/4; a 3-beat loop against a 4-beat loop *is* polymeter. This buys odd meters and polymeter with zero new model concepts, generalizing today's per-track `len`. A real meter/ruler-grouping feature can come later without touching stored data.
- **Polymeter**: tracks keep independent `loop_ticks` and wrap independently (today: `step_index % track.len` in `events_at`; tomorrow: `tick % loop_ticks`). The shared ruler + ghost repeats make this *visible* for the first time.
- **Euclidean**: E(k,n) writes notes at `i × 6` ticks and sets `loop_ticks = n × 6` — unchanged semantics, now legible in any view. The premise paying off.
- **Playhead honesty (known issue)**: `Engine.tick` runs up to one lookahead window (~100 ms) ahead of audible time because `advance()` consumes ticks up to the horizon. The grid already has this artifact in `current_step`; a moving line makes it more noticeable. Ship MVP with it; the fix is UI-side (display the last *emitted* tick whose timestamp ≤ now — the scheduler already has timestamped `TimedMidi`) and is a polish story, not core work.

---

## 4. MVP scope

Prerequisites (defined here; engineering detail in the implementation plan §2.2–§2.5):

- **P1 — Canonical note model under the grid.** `Track` becomes a list of `Note { id, start_tick, len_ticks, pitch, velocity }` plus `loop_ticks`; `events_at` loses the `tick % tps` early-return; `gate_ticks` widens to u32; playback never reads presentation state. The existing grid API survives as an adapter over notes (grid cell *i* ⇔ a note starting exactly at `i × 6`), so the grid UI ships unmodified and the existing test suite plus a golden MIDI-output harness act as the migration safety net. Off-lattice notes are invisible from the grid but persist losslessly across view toggles.
- **P2 — Note CRUD with stable IDs.** Engine surface `add_note` / `update_note` (atomic move+resize+repitch) / `remove_note` / `set_loop_ticks`, enforcing the overlap rule (§5). Note IDs are per-track, monotonic, never reused — indices break mid-drag.
- **P3 — ViewModel and audition.** `ViewModel` carries per-track `notes` (including dormant out-of-loop notes), `loop_ticks`, resolved lane kind, and a global `current_tick`; `audition` is an engine capability (§5) exposed through wasm in the same timed-record format as playback.

Then:

**T1 — View toggle + read-only timeline** *(one sitting)*
As a musician, I switch to timeline view and watch my existing pattern play.
- Toggle `Grilla | Timeline`; switching is instant, lossless both ways, allowed during playback.
- Shared ruler sized to the longest loop; every track renders as a lane (drum or pitch) with notes at correct tick positions/lengths; euclid lanes marked locked; loop brackets and ghost repeats drawn; playhead animates via the existing rAF `render()` loop in `main.ts`.
- Acceptance: a pattern built in the grid looks rhythmically identical on the timeline; MIDI output is byte-identical in both views.

**T2 — Create, delete, audition, snap** *(one sitting)*
As a musician, I put notes down anywhere and hear them instantly.
- Click-drag create (drag = duration; click = one snap unit; with snap off, 6 ticks), right-click/long-press delete; snap `1/16 / 1/8 / 1/4 / off`; created/clicked notes audition to the MIDI output without transport (semantics in §5).
- Acceptance: with snap off I can place a note 2 ticks after a beat and hear it offset on playback; deleting a sounding note never hangs its note-off.

**T3 — Move and resize** *(one sitting)*
As a musician, I refine timing, pitch and duration by dragging, while the loop plays.
- Body drag moves time+pitch; edge drag resizes; engine overlap rule enforced with snap-back feedback.
- Acceptance: edits audible next pass with transport running; no hung notes (core-tested); a note resized across the loop boundary still gets its off (already guaranteed by `due_tick`, verify with a test).

**T4 — Per-track loop length in beats** *(one sitting)*
As a musician, I drag the bracket to make a 3-beat bass phase against 4-beat drums.
- Bracket drag → `set_loop_ticks`, beat-snapped in the UI (1–32 beats; core stores ticks, §3); shortening is non-destructive (out-of-loop notes dimmed, silent); ghost repeats update. Euclid tracks: bracket locked (§2).
- Acceptance: 3-against-4 audibly phases and visibly realigns on the ruler every 3 bars.

**Not in MVP**: zoom/scroll, velocity editing (model stores it; default 100), CC/automation lanes, per-note channel, MIDI recording, song mode/pattern chaining, vertical pitch zoom, playhead-lookahead correction, undo and persistence — with the caveat that **undo then persistence are the first post-MVP stories**, since drag editing is destructive in a way toggles never were and timeline patterns are expensive to rebuild after a reload.

---

## 5. Settled behaviors

Agreed with the implementation plan (2026-06-12); its §2.3–§2.4 and Appendix carry the engineering detail.

- **Overlap rule (same pitch, same track, engine-enforced):** a note's *start* may not fall inside another same-pitch note's span, and two same-pitch notes may not share a start — violating edits are rejected and the UI snaps the drag back with a brief flash. A note's *duration* is clipped to the gap before the next same-pitch note's start, so create-drag and resize always succeed once the start is legal. Different pitches coexist freely. Wrap-around overlap across the loop seam is not policed in MVP — worst case is an early cutoff, identical to today's grid with long gates; the note-off is still guaranteed. Revisit clipping if it feels wrong in play-testing.
- **Audition semantics (engine capability, transport-independent):** clicking or creating a note emits an immediate note-on on the track's *effective* output channel (channel mode applied), at the note's velocity, with a time-based note-off that fires even with the transport stopped and under external clock — auditioned notes can never hang. Audition length is the note's length clamped to 1 beat: a 4-bar pad previews as a 1-beat tap; full length is what playback is for.
- **Default created-note length with snap off:** 6 ticks (one sixteenth).
- **Lane-kind inference:** a track renders as a pitch lane iff it holds ≥2 distinct pitches, else as a drum lane; a per-track override in the gear menu wins. Presentational only — playback never reads it. The track-note control (which repaints every note's pitch, drum-lane semantics) is shown on drum lanes only; on a pitch lane it would silently flatten a melody, with no undo in MVP.
- **Grid view of long loops (MVP caveat):** the grid keeps its 16 cells and shows only the first 16 lattice steps of a longer loop, with a hint that the loop extends beyond; the timeline is the view for long loops. Not a bug to fix in MVP.
- **Playhead lookahead artifact:** shipped as-is (§3, "playhead honesty"); the UI-side correction is a queued post-MVP story.

---

## 6. Trade-offs

### Web-now choices

- **Render lanes on `<canvas>`, keep lane heads as DOM.** The grid's one-button-per-cell DOM doesn't survive tick-resolution blocks, ghost repeats, and a 48 Hz playhead. One canvas per lane (heads stay the existing DOM controls) keeps layout/responsive CSS simple; hit-testing is ~30 lines of TS. Resist a charting/canvas library — the drawing is rectangles and lines.
- **Pointer Events from day one** (not mouse events), so long-press-delete and touch drags work on tablets without a second implementation. The current CSS is already responsive; lanes shrink gracefully because geometry is computed, not laid out.
- **All view state (zoom-future, snap, lane kind, selection, drag) lives in `web/`**, never in core. The only core additions are model-level: notes, `loop_ticks`, `current_tick`, lane-kind *hint* in `TrackView` (presentational tag, like `StepMode` today — playback must never read it, grep-level acceptance check).

### Keeping the ESP32 door open (without constraining the web MVP)

- The firmware consumes the same `ViewModel`: ticks, note lists, `loop_ticks`, `current_tick` — all renderable on a 128×64 OLED as a one-lane-at-a-time compressed strip with encoder-driven cursor. Nothing in this spec assumes pixels, pointer input, or screen width in core.
- The engine-side overlap rule and audition-as-engine-capability (§5) mean the future hardware editor inherits correct behavior instead of reimplementing it.
- **Deliberately deferred, correctly**: hardware encoder gesture mapping, OLED lane rendering, `input.rs` button vocabulary extensions for note editing. Per the web-first scope these wait until the user asks — but note that `handle_input`'s `Button::Step(n)` vocabulary is grid-shaped; when hardware UI work resumes, it needs a representation-agnostic input layer. Don't fix it now; just don't build *more* on it.
- The one real risk to the port is letting the web UI's editing semantics (e.g., what a "snap unit" is, conversion rules) drift into TS-only logic. Mitigation is structural: anything that decides *what the data becomes* goes through the wasm API into core; anything that decides *what the screen shows* stays in TS.

### What I'd push back on

- **Per-lane time scaling** (rejected above) — it's less code but breaks the representation's one promise.
- **A time-signature/meter system** — tick-stored loop lengths with a beat-snapped bracket cover the musical need at near-zero model cost.
- **Starting the circle view next.** Finish two coexisting views over the canonical model first; the third view is cheap *after* the lens-switching pain points are real.

---

**Key files referenced:** `docs/engineering/timeline-view-implementation-plan.md`, `crates/obrero-core/src/pattern.rs` (`events_at`, `Track.len`, `PPQN`), `crates/obrero-core/src/engine.rs` (`pending_offs`, `advance`, `view()`), `crates/obrero-core/src/view.rs`, `web/src/ui.ts`, `web/src/main.ts`.
