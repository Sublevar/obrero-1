# USB-MIDI hardware — Product/design spec

**obrero-1, ESP32-S3 target.** First hardware milestone after the web-first phase. Author: PO agent, 2026-06-12. Status: proposal for review.

Companion docs: `README.md` (§ "USB-MIDI + debug serial"), `MIDI-DIN.md` (DIN hardware, explicitly out of scope here), `docs/product/timeline-view-design-spec.md` (web representation work, unaffected).

---

## 1. Concept

The ESP32-S3 board becomes a **class-compliant USB MIDI device named "obrero-1"**: plug it into any computer, tablet, or USB MIDI host and it appears as a MIDI port with no driver install. The sequencer core (`crates/obrero-core`) runs on-device and emits MIDI clock, transport, and notes over USB exactly as the web version emits them over Web MIDI. **Serial logging stays fully available to the developer at all times**, on a separate physical port.

This milestone deliberately ships **no on-device musical UI** (no display, no encoders, no step buttons beyond a single play/stop control). The product question it answers is narrow: *can the core keep musician-grade time on the hardware target, visible to a DAW, while remaining debuggable?* Everything else builds on that foundation.

### Why this is the right first hardware slice

- It exercises the architecture promise — same `Engine`, different platform shell — with the smallest possible platform surface: a clock (`esp_timer_get_time`), one output (TinyUSB MIDI), one input (a button), one log channel (UART0).
- It is independently useful to a musician: a rock-solid hardware master clock + pattern player for a DAW or iPad rig is a real device, not a tech demo.
- It defers all hardware UI questions (which the representation-agnostic product philosophy says we should not rush) without blocking them.

---

## 2. Goals and non-goals

### Goals (this milestone)

1. Device enumerates as a **USB MIDI class-compliant device** on the S3's native USB port (GPIO19/20) via TinyUSB — recognized by macOS, Windows, Linux, iPadOS with zero drivers.
2. Sensible identity everywhere a musician looks: device and MIDI port named **"obrero-1"** in DAWs and MIDI utilities.
3. The on-device `Engine` plays a pattern as internal clock master: **MIDI Clock (0xF8) at 24 PPQN, Start (0xFA), Stop (0xFC), Note On/Off** with per-track channels — the same byte stream the web version produces.
4. **Timing quality**: clock stable enough that a DAW slaved to the device reports a steady BPM (acceptance criteria in §6).
5. **Logging over serial at all times**: boot logs, panics, and runtime logs on UART0 through the devkit's USB-UART bridge connector; `dev.sh` flash + monitor workflow unchanged.
6. All sequencing logic stays in `obrero-core`. The firmware crate contains only: time source, TinyUSB I/O, the button GPIO, logging, and task plumbing.

### Non-goals (explicitly not this milestone)

- **On-device pattern editing or display.** No screen, no step buttons, no encoder. The starter pattern (`Pattern::starter()`) is the content. Editing arrives with a later "device UI" or "web-edits-device" milestone (§7).
- **MIDI DIN output.** `MIDI-DIN.md` stands as the plan; not wired into this milestone. The firmware design must not preclude adding a second sink later (§7).
- **USB MIDI IN / external clock slave.** The core already supports it (`feed_midi_in`, `ClockSource::External`) — it is the designated fast-follow (§5, M2), not MVP.
- **Composite USB device (CDC console on the same cable).** Evaluated and rejected for now — see decision in §4.
- **USB host mode** (device powering/hosting a keyboard), battery/standalone power UX, Wi-Fi/BLE MIDI.
- **ESP32 classic support for USB-MIDI.** No USB-OTG on that chip; it stays a DIN-only future target (README already documents commenting out the TinyUSB component for it).

---

## 3. User stories

### Musician

**U1 — Plug and play into a DAW.**
As a musician, I plug the obrero-1 into my laptop with one USB cable and it shows up in Ableton/Logic/Reaper as a MIDI input port named "obrero-1", with no driver installation.
*Acceptance:*
- Device enumerates on macOS, Windows 10+, and Linux (ALSA) as a class-compliant MIDI device.
- Port name shown by the DAW contains "obrero-1" (not "TinyUSB", not "Espressif device") on all three OSes.
- Time from cable-in to usable port < 3 s.

**U2 — Hardware master clock.**
As a musician, I press play on the device and my DAW (set to external sync) locks to its clock; my drum machine downstream of the DAW follows.
*Acceptance:*
- On play: Start (0xFA) then Clock (0xF8) at 24 PPQN at the engine BPM; on stop: Stop (0xFC) and all sounding notes get Note Off (engine `flush_pending_offs` behavior preserved).
- DAW slaved at 120 BPM displays 120.0 ± 0.1 BPM, no drift over 5 minutes.

**U3 — Pattern audible immediately.**
As a musician, I route the "obrero-1" port to a software synth and hear the starter pattern with correct channels and velocities, identical in content to what the web version plays.
*Acceptance:*
- Note On/Off bytes match `obrero-core` output for the same pattern/BPM (verifiable in a MIDI monitor).
- A single physical button (devkit BOOT button, GPIO0) toggles play/stop via the core's `InputEvent::ButtonDown(Button::Play/Stop)` path. Stop never leaves hung notes.

**U4 — Predictable identity across sessions.**
As a musician, my DAW remembers the device routing between sessions and between USB ports.
*Acceptance:* USB serial-number string is stable (derived from the chip's eFuse MAC), so the host treats it as the same device on any port.

### Developer

**D1 — Logs while MIDI runs.**
As the developer, I watch live logs (transport changes, enumeration state, errors) over the UART connector while the native USB port is busy being a MIDI device.
*Acceptance:*
- `./dev.sh` flashes and monitors via `/dev/ttyUSB*` with no changes to the script's contract.
- Log output includes: boot banner, TinyUSB mount/unmount/suspend events, play/stop transitions with BPM, and any dropped-write warnings.
- Logging never blocks the MIDI scheduler task (see §6 rate-limit rule).

**D2 — Crash visibility.**
As the developer, when the firmware panics or the USB stack wedges, I still see the panic backtrace and early-boot logs on serial.
*Acceptance:* a deliberate `panic!()` test build prints its backtrace on UART0; nothing about panic visibility depends on TinyUSB being alive.

**D3 — Timing self-check.**
As the developer, I can enable a debug log mode that reports scheduler health (max lateness per window, dropped writes) so timing regressions are measurable without a MIDI analyzer.
*Acceptance:* a compile-time or log-level switch emits a once-per-second summary line (never per-tick at default level).

---

## 4. USB architecture decision

### Decision: **MIDI-only on the native USB port; logs and flashing on UART0 (two cables for the developer, one cable for the musician).**

This formalizes what `firmware/sdkconfig.defaults` and `dev.sh` already sketch. The alternative — a composite CDC+MIDI device on a single cable — is rejected for this milestone.

### Context (hardware constraint)

The S3 has one internal USB PHY shared between the **USB-OTG** controller (what TinyUSB drives) and the **USB-Serial-JTAG** controller (what gives "free" serial over the native port). Using TinyUSB means USB-Serial-JTAG is gone — logging must go *somewhere else*. The two candidate homes:

| | **A. MIDI-only + UART0 logs (chosen)** | **B. Composite CDC+MIDI on one cable** |
|---|---|---|
| End-user enumeration | Clean: one MIDI device, nothing else | MIDI **plus** a mystery serial port in every OS; Linux ModemManager may probe it; "what is this COM port?" support noise |
| Panic / early-boot logs | Always visible (UART0 is the ESP-IDF console from the first boot ROM message) | Lost: CDC console only works after TinyUSB is up, and dies exactly when the firmware crashes — the moment you need it most |
| Flashing | `espflash` over the UART bridge with auto-reset, regardless of firmware state | Native-port flashing needs manual download-mode (BOOT button dance) once OTG owns the PHY; a bricked USB stack means no reflash without it |
| Dev workflow | `dev.sh` unchanged; monitor never fights the MIDI port | espflash monitor vs. CDC console interleaving; tooling churn |
| Cables | Two for the developer (devkit has both connectors anyway); **one for the musician** — the UART cable is dev-only | One for everyone |
| Host compatibility | Maximal — plain MIDI class device is the most-tested enumeration path everywhere, incl. iPadOS camera-adapter rigs | Composite is well-supported on modern OSes (ESP-IDF's TinyUSB explicitly supports CDC+MIDI composite) but adds an interface iPadOS and class-compliant hardware hosts must ignore |
| Future console feature | Can be added later behind a feature flag without breaking identity (see note below) | Locked in from day one |

The single-cable convenience of B benefits only the developer — the musician never wants the serial port — and it costs the developer the two things they most need from a debug channel: crash visibility and recovery flashing. A second cable into connectors the devkit already has is the cheapest robustness available.

**Future note:** if a *product* feature later wants a host-side data channel (pattern sync with the web app, a config CLI), that conversation reopens — but the recommendation there is **SysEx over the existing MIDI interface**, not CDC: it keeps single-interface enumeration, works from the browser via Web MIDI (no Web Serial permission dance), and reuses the channel musicians already authorized. CDC remains the fallback if throughput demands it. Caveat to plan for: adding interfaces or changing descriptors later should be paired with a PID bump or interface-order discipline, because Windows caches enumeration per VID/PID.

### Device identity

| Descriptor | Value | Rationale |
|---|---|---|
| VID/PID | Espressif VID `0x303A` + a PID from Espressif's public allocation process (open to projects using their silicon); interim dev builds use their test PID range | obrero-1 is GPL hobby hardware; a $6k USB-IF VID is out of the question; Espressif's program is the standard route for ESP32 projects |
| Manufacturer string | `Sublevar` | Matches repo identity |
| Product string | `obrero-1` | What macOS/Windows/ALSA derive port names from |
| MIDI jack/interface strings | `obrero-1` (same) | Hosts differ in which string they surface for the *port* (Windows WinMM truncates the product string at 31 chars; macOS may use jack strings); setting all of them identically is the only way to get U1's "named obrero-1 everywhere". Requires passing a custom `string_descriptor` array in `tinyusb_config_t` rather than relying on component defaults, which would otherwise surface as "TinyUSB MIDI" |
| Serial string | From eFuse MAC | U4 stable identity |
| Strings config | Prefer `sdkconfig.defaults` (`CONFIG_TINYUSB_DESC_*`) where the component exposes it; custom descriptor array where it doesn't (MIDI jack strings) | Keep config declarative and diff-able |

### Port behavior (defined product behavior, not incidental)

- **One MIDI OUT port (device→host "MIDI IN" in the DAW), one virtual cable.** The descriptor also carries the IN direction (standard for MIDI class devices); until M2, received data is read and discarded to keep endpoints healthy.
- **Host disconnect / USB suspend while playing:** transport stops, pending note-offs flushed into the (dead) stream, engine returns to Stopped. Rationale: with no local UI and no other sink, "playing into the void" only produces hung notes on reconnect. When DIN OUT exists (§7), this rule is revisited — by then the device playing standalone is meaningful.
- **Reconnect:** device re-enumerates; transport stays stopped; musician presses play. No auto-resume in MVP (surprise notes are worse than a button press).
- **TX backpressure:** if the TinyUSB FIFO is full (host stalled), realtime bytes (clock) get priority; note events drop with a rate-limited warning log rather than blocking the scheduler. Dropped-event counters surface in the D3 health line.

---

## 5. Scope: MVP and what comes after

### M1 — MVP (this spec's commitment)

1. **S1 — Enumerate with identity.** TinyUSB driver install with custom descriptors per §4; acceptance = U1 + U4 on the three desktop OSes.
2. **S2 — Engine on a real clock.** A dedicated high-priority FreeRTOS task (pinned core, e.g. core 1) owning the `Engine`: sleep until `next_event_at()`, call `advance(now_us, 0, out)` (lookahead 0, per the engine's own doc comment — USB-MIDI writes are immediate, unlike Web MIDI's timestamped sends), write each `TimedMidi` via `tud_midi_n_stream_write`. Replaces the scaffold tasks in `firmware/src/main.rs`.
3. **S3 — Transport button.** BOOT button (GPIO0, debounced) → `handle_input(ButtonDown(Play/Stop))` toggle; acceptance = U3.
4. **S4 — Serial logging discipline.** UART0 console (already `CONFIG_ESP_CONSOLE_UART_DEFAULT=y`); log events per D1; per-tick logging forbidden above debug level; D3 health line.
5. **S5 — Disconnect/suspend safety.** Mount/unmount/suspend callbacks → stop transport, flush offs; acceptance per §4 port behavior.
6. **S6 — Timing validation.** The §6 acceptance procedure run and recorded (numbers in the PR description, not a doc).

Each story is sized to be shippable in one sitting given the component and bindings already in place.

### M2 — Fast-follow (committed direction, not in MVP)

- **USB MIDI IN → external clock slave.** Read host bytes (`tud_midi_n_stream_read`), feed `engine.feed_midi_in()`; a way to switch `ClockSource` (long-press the button, or host SysEx). The core needs zero changes — this is why it's cheap and why it's not allowed to bloat M1.
- **Continue (0xFB) emission as master.** The engine currently restarts from tick 0 on `play()` and never emits Continue; pause/continue semantics belong in `obrero-core` (both targets benefit) and should land there, not be faked in firmware.

### Deferred (explicitly not yet, with reasons)

- **Pattern editing on device / from web app via SysEx** — the highest-value next product step after M2, but it needs its own spec (protocol versioning, pattern serialization shared with the web's save format). Don't smuggle a protocol into M1.
- **DIN MIDI OUT alongside USB** (`MIDI-DIN.md`) — wait until the USB path proves the timing core; then DIN is "second sink for the same `TimedMidi` stream", and the §4 disconnect rule gets revisited.
- **Composite CDC** — see §4; reopen only against a concrete product need that SysEx can't serve.
- **Any on-device representation UI** — screens/encoders deserve the same representation-agnostic thinking as the web views; premature now.
- **PSRAM enablement** — nothing in this milestone needs it; the README documents how when something does.

---

## 6. Timing & latency expectations

- **Budget context:** USB full-speed delivers MIDI within 1 ms frames; classic DIN takes ~1 ms per 3-byte message anyway. The musician-meaningful bar is "clock a DAW can lock to and notes that don't flam".
- **Targets:** scheduler lateness (actual write time vs. `TimedMidi.at_us`) ≤ 0.5 ms typical, ≤ 1 ms worst-case under logging load; zero dropped clock bytes in normal operation.
- **Acceptance procedure (S6):** record the device's clock and notes in a DAW for 5 min at 120 BPM and at 300 BPM (engine `MAX_BPM`, worst tick density: 120 ticks/s); DAW-derived BPM stable ± 0.1; inter-clock interval jitter inspected; D3 health line corroborates.
- **Design rules to hit it:** MIDI task isolated on its own core at high priority; logging from other tasks only; no heap allocation in the tick path (reuse the `Vec<TimedMidi>` buffer); never call blocking log macros from the scheduler task.

## 7. Open questions

1. **PID allocation timing** — apply to Espressif's program now (name is public anyway) or ship dev builds on the test PID until first hardware goes to a stranger?
2. **Clock-source switching UX in M2** — long-press on the single button vs. host-side command? Leaning long-press (works without a computer-side tool), but it's M2's call.
3. **Boot behavior** — should the device auto-play on power-up once DIN out exists (groovebox expectation) while staying manual-start when USB-only? Revisit with the DIN milestone.
4. **Does `tinyusb_config_t` string array fully cover MIDI jack naming on macOS?** Needs a bench check on a Mac early in S1 — if Core MIDI surfaces jack strings we can't set via the component, the fix is a fully custom descriptor block, which is fine but should be discovered in week one, not at the end.

---

Sources:
- [ESP-IDF USB Device Stack (ESP32-S3) — composite CDC+MIDI support, `tinyusb_config_t` descriptors and string configuration](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/api-reference/peripherals/usb_device.html)
