import { BN } from "@coral-xyz/anchor";

type AmountInput = BN | string | bigint;

function toBigint(v: AmountInput): bigint {
  if (typeof v === "bigint") return v;
  if (typeof v === "string") return BigInt(v);
  return BigInt(v.toString());
}

/** Format a base-unit token amount using string/BN math (default 6 decimals). */
export function formatAmount(v: AmountInput, decimals = 6): string {
  const raw = toBigint(v);
  const base = 10n ** BigInt(decimals);
  const whole = raw / base;
  const frac = raw % base;
  const fracStr = frac.toString().padStart(decimals, "0").replace(/0+$/, "");
  return fracStr.length === 0 ? whole.toString() : `${whole}.${fracStr}`;
}

/** Parse a decimal string into base units (string in, string out — no floats). */
export function parseAmount(decimalStr: string, decimals = 6): string {
  const s = decimalStr.trim();
  if (!/^\d+(\.\d+)?$/.test(s)) throw new Error("Amount must be a positive decimal");
  const [w, f = ""] = s.split(".");
  if (f.length > decimals) throw new Error(`Too many decimals (max ${decimals})`);
  const frac = f.padEnd(decimals, "0");
  const raw = BigInt(w) * 10n ** BigInt(decimals) + (frac ? BigInt(frac) : 0n);
  if (raw <= 0n) throw new Error("Amount must be above zero");
  return raw.toString();
}

export function shortAddress(addr: string | { toBase58(): string }): string {
  const s = typeof addr === "string" ? addr : addr.toBase58();
  if (s.length <= 12) return s;
  return `${s.slice(0, 4)}…${s.slice(-4)}`;
}

/** Unix seconds (BN|string|number) to locale string. */
export function formatTimestamp(secs: BN | string | number | bigint): string {
  const n = Number(secs.toString());
  if (!n) return "—";
  return new Date(n * 1000).toLocaleString();
}

/** Seconds remaining to mm:ss / hh:mm:ss / Xd Xh form. Negative clamps to "elapsed". */
export function formatCountdown(deadlineSecs: number, nowSecs?: number): string {
  const now = nowSecs ?? Math.floor(Date.now() / 1000);
  const diff = Math.floor(deadlineSecs - now);
  if (diff <= 0) return "elapsed";
  const d = Math.floor(diff / 86400);
  const h = Math.floor((diff % 86400) / 3600);
  const m = Math.floor((diff % 3600) / 60);
  const s = diff % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}:${pad(m)}:${pad(s)}`;
  return `${m}:${pad(s)}`;
}

/** Review deadline = submittedAt + windowSecs (all BN-ish). */
export function reviewDeadline(
  submittedAt: BN | string | number | bigint,
  windowSecs: BN | string | number | bigint
): number {
  return Number(submittedAt.toString()) + Number(windowSecs.toString());
}
