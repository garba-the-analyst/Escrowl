"use client";
import { BN } from "@coral-xyz/anchor";

import { useEffect, useState } from "react";
import type { Milestone } from "@escrowl/sdk/src/types";
import { milestoneStatus } from "@escrowl/sdk/src/types";
import { StatusBadge } from "./status-badge";
import { formatAmount, formatCountdown, formatTimestamp, reviewDeadline } from "@/lib/format";

export type MilestoneAction =
  | "submit"
  | "approve"
  | "claim"
  | "dispute"
  | "resolve"
  | "reclaim"
  | "expire";

export interface ActionPermission {
  allowed: boolean;
  reason?: string;
}

export function MilestoneTimeline({
  milestones,
  reviewWindowSecs,
  permissions,
  pendingIndex,
  onAction,
}: {
  milestones: Milestone[];
  reviewWindowSecs: BN;
  permissions: Record<MilestoneAction, (index: number) => ActionPermission>;
  pendingIndex: number | null;
  onAction: (action: MilestoneAction, index: number) => void;
}) {
  const [, setNow] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    const t = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(t);
  }, []);

  const actions: { key: MilestoneAction; label: string }[] = [
    { key: "submit", label: "Submit" },
    { key: "approve", label: "Approve" },
    { key: "claim", label: "Claim" },
    { key: "dispute", label: "Dispute" },
    { key: "resolve", label: "Resolve" },
    { key: "reclaim", label: "Reclaim" },
    { key: "expire", label: "Expire" },
  ];

  return (
    <ol className="flex flex-col gap-3">
      {milestones.map((m, i) => {
        const status = milestoneStatus(m);
        const submitted = Number(m.submittedAt.toString());
        const deadline =
          submitted > 0 ? reviewDeadline(m.submittedAt, reviewWindowSecs) : null;
        const ev = m.evidenceHash?.length
          ? `0x${m.evidenceHash.slice(0, 4).map((b) => b.toString(16).padStart(2, "0")).join("")}…`
          : "—";
        return (
          <li key={i} className="card p-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="font-semibold">
                #{i + 1} · {formatAmount(m.amount)}
              </span>
              <StatusBadge status={status} />
            </div>
            <div className="mt-1 text-xs" style={{ color: "var(--muted)" }}>
              <p>
                Submitted: {submitted > 0 ? formatTimestamp(m.submittedAt) : "—"}
                {deadline !== null && status === "submitted" && (
                  <> · deadline in {formatCountdown(deadline)}</>
                )}
              </p>
              <p>Evidence: {ev}</p>
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              {actions.map((a) => {
                const p = permissions[a.key](i);
                const busy = pendingIndex === i;
                return (
                  <button
                    key={a.key}
                    className="btn"
                    disabled={!p.allowed || busy}
                    title={p.allowed ? a.label : p.reason ?? "Not permitted"}
                    onClick={() => onAction(a.key, i)}
                  >
                    {busy ? "…" : a.label}
                  </button>
                );
              })}
            </div>
          </li>
        );
      })}
    </ol>
  );
}
