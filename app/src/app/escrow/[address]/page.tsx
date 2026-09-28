// Server wrapper: prerenders seeded escrows for the static export.
import EscrowDetailClient from "./detail-client";

// Seeded devnet escrows, prerendered for the static export (GitHub Pages).
// New escrows resolve client-side from the dashboard.
export function generateStaticParams() {
  return [
    "huZokHPQ7kF29sZZ1EiCkPvRCcfYvMunwSa1jhtiyTu",
    "FKTp1chn8GE76W9BRXqo4bJgkvG9tKKsUTmh8YakD93r",
    "J588Ai9rKmiqn3PTdozUXK7mBXxTESgaUXYzwxMdA6zC",
    "7UvwNb9wg8nKDzFFFR3xQQ1qKeKqVKjr5sTpAaZiF49H",
    "C8QvcBZSfYNrrhuV7NcadXk5tfEYFU2wmb6yB7pQPz3h",
  ].map((address) => ({ address }));
}

export default async function EscrowDetailPage({
  params,
}: {
  params: Promise<{ address: string }>;
}) {
  const { address } = await params;
  return <EscrowDetailClient address={address} />;
}
