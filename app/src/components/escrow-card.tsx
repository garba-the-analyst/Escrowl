"use client";
import type { PublicKey } from "@solana/web3.js";

import Link from "next/link";
import type { EscrowAccount } from "@escrowl/sdk/src/types";
import { escrowStatus, milestoneStatus } from "@escrowl/sdk/src/types";
import { StatusBadge } from "./status-badge";
import { formatAmount, shortAddress } from "@/lib/format";

export function EscrowCard({
  address,
  account,
}: {
  address: PublicKey;
  account: EscrowAccount;
}) {
  const status = escrowStatus(account);
  const total = account.milestones.length;
  const done = account.milestones.filter((m) =>
    ["released", "resolved", "refunded"].includes(milestoneStatus(m))
  ).length;
  const pct = total === 0 ? 0 : Math.round((done / total) * 100);

  return (
    <Link
      href={`/escrow/${address.toBase58()}`}
      className="card block p-4 hover:opacity-90"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="font-mono text-sm font-semibold">
          {shortAddress(address.toBase58())}
        </span>
        <StatusBadge status={status} />
      </div>
      <div className="mt-2 text-xs" style={{ color: "var(--muted)" }}>
        <p>Counterparty: {shortAddress(account.seller.toBase58())}</p>
        <p>
          Total: {formatAmount(account.totalAmount)} · {done}/{total} milestones
        </p>
      </div>
      <div
        className="mt-3 h-2 w-full rounded-full"
        style={{ background: "var(--border)" }}
      >
        <div
          className="h-2 rounded-full"
          style={{ width: `${pct}%`, background: "var(--accent)" }}
        />
      </div>
    </Link>
  );
}
