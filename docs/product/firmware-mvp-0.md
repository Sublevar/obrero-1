# Firmware MVP-0 — Platform validation slice

**obrero-1, ESP32-S3.** Predecessor to M1 (`docs/product/usb-midi-hardware.md`). Status: proposal. Author: PO agent, 2026-06-12.

## 1. What MVP-0 is

The thinnest firmware that answers one question: **does our platform configuration work end to end?** Toolchain → esp_tinyusb bindings → USB-MIDI enumeration on the native OTG port → bytes reaching a DAW → UART0 logs alive the whole time → `dev.sh` flash/monitor unaffected.

It contains **no sequencer**. `obrero-core` is **not** linked. Rationale: the engine is already proven on the web target; linking it here retires zero platform risk while adding build surface and a second suspect when something fails. If MVP-0 misbehaves, every symptom is a platform symptom. Engine integration is exactly S2's job.

The question decomposes into five risks MVP-0 must retire:

| # | Risk | Retired when |
|---|---|---|
| R1 | **Bindings build pipeline** — esp_tinyusb component + `tinyusb_bindings.h` produce usable Rust bindings (`tinyusb_driver_install`, `tud_midi_n_stream_write`) on the xtensa toolchain | Firmware compiles, links, and calls them successfully |
| R2 | **Enumeration from Rust** — TinyUSB driver installed from Rust enumerates a class-compliant MIDI device on GPIO19/20 | Host shows a MIDI port, no drivers |
| R3 | **Descriptor/identity config path** — we can override component-default strings (the §4 mechanism works at all) | Port name contains "obrero-1", not "TinyUSB MIDI" |
| R4 | **UART0 console coexists with OTG** — logs, boot banner, panic output on `/dev/ttyUSB*` while OTG is busy being MIDI | Live logs stream during MIDI activity; reset shows full boot log |
| R5 | **Flash/monitor workflow with OTG owning the PHY** — `dev.sh` contract holds, including re-flash while enumerated | Re-flash succeeds with the MIDI cable connected and notes playing |

## 2. Observable behavior: a fixed metronome note

On boot, the device enumerates and immediately, forever, emits **Note On (ch 1, C4, vel 100) every 500 ms, Note Off 250 ms later** — a 120 BPM quarter-note metronome — plus a once-per-second log line on UART0 (`beat N, uptime X ms`). Timing from `esp_timer_get_time`, no FreeRTOS-tick sleeps as the time base. No button, no clock messages, no MIDI IN handling beyond draining reads.

**Why a periodic note and not the alternatives:**

- *Silent enumeration* proves R1–R3 but leaves the actual TX path (R5's data half, FIFO behavior, write API correctness) unproven — and that path is where binding bugs hide.
- *MIDI clock at 24 PPQN* exercises the same write API but is inaudible, requires DAW external-sync setup to observe, and its throughput-stress value belongs with S6's rigor, not here.
- *A note every beat* is verifiable in ten seconds with any soft synth, proves the full device→host data path, both Note On **and** Note Off (hung notes = bug), and a human ear plus a MIDI monitor's timestamps give a free, crude first read on jitter — enough to flag a gross platform problem (e.g. tick-quantized sleeps) before S6 measures properly.

Maximum validation signal, minimum logic: one timer, one 3-byte message pair, one log line.

## 3. Acceptance checklist (one devkit, laptop, two cables)

1. **[R1]** `cargo build` succeeds; the `tinyusb` bindings module resolves with no manual steps beyond the repo's documented setup.
2. **[R5]** `./dev.sh` flashes via `/dev/ttyUSB*` and opens the monitor, with the native USB cable **disconnected**. Boot banner visible.
3. **[R2]** Connect the native USB cable to the laptop: a MIDI input port appears within 3 s (`amidi -l` / DAW MIDI settings). No driver prompt, no error in `dmesg`.
4. **[R3]** The port name as shown by ALSA/the DAW contains **"obrero-1"**. (Linux only for MVP-0; the three-OS matrix is S1.)
5. **[TX path, R2/R5]** Route the port to a soft synth: steady metronome tone, no hung notes over 2 minutes; a MIDI monitor shows alternating Note On/Off, ch 1, note 60, with inter-onset spacing eyeballing to ~500 ms.
6. **[R4]** While step 5 plays, the UART monitor streams the per-second log line; neither stream stutters the other.
7. **[R4]** Press the devkit RESET button: full early-boot log appears on UART, device re-enumerates, metronome resumes.
8. **[R5]** With the device enumerated and playing, run `./dev.sh` again: re-flash succeeds without touching the BOOT button or unplugging the MIDI cable.

All eight pass → MVP-0 done, platform risks retired, M1 starts.

## 4. Deferred to M1 (and what MVP-0 covers of S1–S6)

Explicitly **not** in MVP-0: `obrero-core` integration and the real scheduler task (S2); play/stop button (S3); logging discipline rules and D3 health line (S4 — MVP-0's log line is a placeholder, not the contract); mount/unmount/suspend handling and note-off flush (S5 — MVP-0 may play into the void; acceptable for a bench tool, banned from M1); timing validation at §6 rigor (S6); full identity — serial-from-eFuse-MAC, MIDI jack strings, macOS/Windows verification, PID decision (rest of S1); TX backpressure policy; everything already deferred by the M1 spec.

**Coverage:** MVP-0 partially covers **S1** (happy-path enumeration + product-string mechanism proven; identity matrix remains) and the substrate of **S4** (UART console demonstrably coexists with OTG). It replaces nothing. Suggested order after MVP-0: **S1-completion → S2 → S3 → S4 → S5 → S6** — identity first while the firmware is still trivial (Windows descriptor caching makes late identity changes painful), then engine, then safety, then measurement.

## 5. Scaffolding: keep vs. throw away

**Keep unchanged:** `firmware/Cargo.toml` (the `extra_components` block is the thing under test), `firmware/src/tinyusb_bindings.h`, `firmware/sdkconfig.defaults`, `firmware/dev.sh`. One addition to sdkconfig is in-scope if R3 requires it: descriptor string config (and consider enabling `CONFIG_FREERTOS_HZ=1000`, already sketched there, if 100 Hz ticks visibly quantize the metronome — that finding itself is platform signal worth having before S2).

**Throw away entirely:** the three demo tasks and print-loop in `firmware/src/main.rs`. They are not just dead weight — `task3` busy-loops without yielding (watchdog/starvation), and the `CString::new(...).unwrap().as_ptr()` task-name pattern passes a dangling pointer. Nothing there is worth salvaging; MVP-0's `main.rs` is: link patches, install TinyUSB driver, one metronome loop, one log line.
