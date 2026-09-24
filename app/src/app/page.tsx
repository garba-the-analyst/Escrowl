"use client";
import type { PublicKey } from "@solana/web3.js";

import { useEffect, useState } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import type { EscrowAccount } from "@escrowl/sdk/src/types";
import { humanError } from "@escrowl/sdk/src/errors";
import { useEscrowlClient } from "@/lib/client";
import { EscrowCard } from "@/components/escrow-card";

type Tab = "buyer" | "seller" | "arbiter";

interface Row {
  publicKey: PublicKey;
  account: EscrowAccount;
}

export default function Dashboard() {
  const { publicKey } = useWallet();
  const client = useEscrowlClient();
  const [tab, setTab] = useState<Tab>("buyer");
  const [rows, setRows] = useState<Row[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      if (!client || !publicKey) {
        setRows([]);
        return;
      }
      setLoading(true);
      setError(null);
      try {
        const res = await client.listEscrows(tab, publicKey);
        if (!cancelled) setRows(res as unknown as Row[]);
      } catch (e) {
        if (!cancelled) setError(humanError(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    run();
    return () => {
      cancelled = true;
    };
  }, [client, publicKey, tab]);

  if (!publicKey) {
    return (
      <div className="card p-6 text-center">
        <h1 className="text-xl font-bold">Connect a wallet to view escrows</h1>
        <p className="mt-2 text-sm" style={{ color: "var(--muted)" }}>
          Dashboard shows escrows where you are buyer, seller, or arbiter.
        </p>
        <div className="mt-4 flex justify-center">
          <WalletMultiButton />
        </div>
      </div>
    );
  }

  const tabs: { key: Tab; label: string }[] = [
    { key: "buyer", label: "As Buyer" },
    { key: "seller", label: "As Seller" },
    { key: "arbiter", label: "As Arbiter" },
  ];

  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-xl font-bold">Dashboard</h1>
        <WalletMultiButton />
      </div>
      <div className="mt-4 flex gap-2">
        {tabs.map((t) => (
          <button
            key={t.key}
            className={`tab ${tab === t.key ? "tab-active" : ""}`}
            onClick={() => setTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="mt-4">
        {loading && <p className="text-sm">Loading escrows…</p>}
        {error && <p className="text-sm" style={{ color: "var(--danger)" }}>{error}</p>}
        {!loading && !error && rows.length === 0 && (
          <div className="card p-6 text-center text-sm" style={{ color: "var(--muted)" }}>
            No escrows found for this role.
          </div>
        )}
        <div className="grid gap-3 sm:grid-cols-2">
          {rows.map((r) => (
            <EscrowCard key={r.publicKey.toBase58()} address={r.publicKey} account={r.account} />
          ))}
        </div>
      </div>
    </div>
  );
}
