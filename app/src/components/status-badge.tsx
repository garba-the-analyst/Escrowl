import type { EscrowStatusKind, MilestoneStatusKind } from "@escrowl/sdk/src/types";

const COLORS: Record<string, string> = {
  created: "#8b949e",
  funded: "#2f81f7",
  completed: "#3fb950",
  cancelled: "#f85149",
  pending: "#8b949e",
  submitted: "#2f81f7",
  released: "#3fb950",
  disputed: "#f85149",
  resolved: "#a371f7",
  refunded: "#d29922",
};

export function StatusBadge({
  status,
}: {
  status: EscrowStatusKind | MilestoneStatusKind;
}) {
  const color = COLORS[status] ?? "#8b949e";
  return (
    <span
      className="inline-block rounded-full px-2 py-0.5 text-xs font-semibold"
      style={{ color, border: `1px solid ${color}` }}
    >
      {status}
    </span>
  );
}
