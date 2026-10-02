// Vigila pérdidas del motor: ticks descartados por ring lleno, ticks y pasos
// salteados tras un atraso, y avisos perdidos por el DebugTap. Los avisa por
// consola y, en `vite dev`, al servidor (terminal de vite). Los avisos se
// pueden dormir; los contadores siguen y el indicador de debug los muestra.

import type { WasmEngine } from "./pkg/obrero_wasm";

const FLUSH_INTERVAL_MS = 250;
const SERVER_URL = "/__obrero/loss";

export interface LossCounters {
  lostTicks: number;
  skippedTicks: number;
  skippedSteps: number;
  lostNotices: number;
  lastGapMs: number;
  maxGapMs: number;
}

export class LossMonitor {
  readonly totals: LossCounters = {
    lostTicks: 0,
    skippedTicks: 0,
    skippedSteps: 0,
    lostNotices: 0,
    lastGapMs: 0,
    maxGapMs: 0,
  };
  muted: boolean;

  private seen = [0, 0, 0];
  private pending = { lostTicks: 0, skippedTicks: 0, skippedSteps: 0, lostNotices: 0 };
  private lastFlush = 0;
  private lastGapAtLoss = 0;

  constructor(muted: boolean) {
    this.muted = muted;
  }

  /** Avisos que el DebugTap perdió por ring lleno. */
  addLostNotices(n: number): void {
    if (n <= 0) return;
    this.totals.lostNotices += n;
    this.pending.lostNotices += n;
  }

  /** Una vez por pump. `gapMs` = tiempo desde el pump anterior. */
  poll(engine: WasmEngine, gapMs: number): void {
    this.totals.lastGapMs = gapMs;
    this.totals.maxGapMs = Math.max(this.totals.maxGapMs, gapMs);

    const now = engine.loss_report();
    const dLost = now[0] - this.seen[0];
    const dTicks = now[1] - this.seen[1];
    const dSteps = now[2] - this.seen[2];
    this.seen = [now[0], now[1], now[2]];

    this.totals.lostTicks = now[0];
    this.totals.skippedTicks = now[1];
    this.totals.skippedSteps = now[2];

    if (dLost || dTicks || dSteps) {
      this.pending.lostTicks += dLost;
      this.pending.skippedTicks += dTicks;
      this.pending.skippedSteps += dSteps;
      this.lastGapAtLoss = gapMs;
    }
    this.flush(performance.now());
  }

  get lossy(): boolean {
    const t = this.totals;
    return t.lostTicks + t.skippedTicks + t.skippedSteps + t.lostNotices > 0;
  }

  private flush(nowMs: number): void {
    const p = this.pending;
    if (!(p.lostTicks || p.skippedTicks || p.skippedSteps || p.lostNotices)) return;
    if (nowMs - this.lastFlush < FLUSH_INTERVAL_MS) return;
    this.lastFlush = nowMs;
    const parts: string[] = [];
    if (p.skippedTicks) parts.push(`${p.skippedTicks} ticks saltados (${p.skippedSteps} pasos sin sonar)`);
    if (p.lostTicks) parts.push(`${p.lostTicks} ticks descartados por ring lleno`);
    if (p.lostNotices) parts.push(`${p.lostNotices} avisos del debug perdidos`);
    const text = `${parts.join(", ")}; pump atrasado ${this.lastGapAtLoss.toFixed(1)} ms`;
    this.pending = { lostTicks: 0, skippedTicks: 0, skippedSteps: 0, lostNotices: 0 };
    if (this.muted) return;
    console.warn(`[obrero] PÉRDIDA: ${text}`);
    this.sendToServer(text);
  }

  private sendToServer(text: string): void {
    if (!import.meta.env.DEV) return;
    try {
      void fetch(SERVER_URL, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ text, totals: this.totals }),
        keepalive: true,
      }).catch(() => {});
    } catch {
      // Sin servidor de desarrollo: nada que avisar.
    }
  }
}
