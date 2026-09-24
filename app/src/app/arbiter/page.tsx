"use client";
import { PublicKey } from "@solana/web3.js";

import { useState } from "react";
import Link from "next/link";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { humanError } from "@escrowl/sdk/src/errors";
import { milestoneStatus } from "@escrowl/sdk/src/types";
import { useEscrowlClient } from "@/lib/client";

export default function ArbiterConsole() {
  const { publicKey } = useWallet();
  const client = useEscrowlClient();
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [rows, setRows] = useState<
    { address: string; disputed: number[]; total: number }[]
  >([]);
  const [sellerAmt, setSellerAmt] = useState<Record<string, string>>({});
  const [buyerAmt, setBuyerAmt] = useState<Record<string, string>>({});
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionOk, setActionOk] = useState<string | null>(null);

  const effective = input.trim() || publicKey?.toBase58() || "";

  async function load() {
    if (!client || !effective) return;
    setLoading(true);
    setError(null);
    try {
      const wallet = new PublicKey(effective);
      const res = await client.listEscrows("arbiter", wallet);
      const filtered = res
        .map((r) => {
          const ms = (r.account.milestones ?? []) as unknown as Parameters<typeof milestoneStatus>[0][];
          const disputed = ms
            .map((m, i) => (milestoneStatus(m) === "disputed" ? i : -1))
            .filter((i) => i >= 0);
          return { address: r.publicKey.toBase58(), disputed, total: ms.length };
        })
        .filter((r) => r.disputed.length > 0);
      setRows(filtered);
    } catch (e) {
      setError(humanError(e));
    } finally {
      setLoading(false);
    }
  }

  async function resolve(address: string, index: number) {
    if (!client || !publicKey) return;
    const key = `${address}:${index}`;
    setBusyKey(key);
    setActionError(null);
    setActionOk(null);
    try {
      const escrow = new PublicKey(address);
      const esc = await client.getEscrow(escrow);
      const ms = esc.milestones[index] as unknown as { amount: { toString(): string } };
      const sRaw = (sellerAmt[key] ?? "").trim();
      const bRaw = (buyerAmt[key] ?? "").trim();
      if (!sRaw || !bRaw) throw new Error("Enter both seller and buyer amounts (base units)");
      const total = BigInt(ms.amount.toString());
      if (BigInt(sRaw) + BigInt(bRaw) !== total)
        throw new Error("Split must add up exactly to the milestone amount");
      // Seller/buyer ATAs resolve from on-chain parties; treasury from the
      // escrow snapshot (rotations only affect future escrows).
      const sellerAta = getAssociatedTokenAddressSync(esc.mint, esc.seller);
      const buyerAta = getAssociatedTokenAddressSync(esc.mint, esc.buyer);
      const treasuryAta = getAssociatedTokenAddressSync(esc.mint, esc.treasurySnapshot);
      await client
        .resolveDispute({
          arbiter: publicKey,
          escrow,
          sellerAta,
          buyerAta,
          treasuryAta,
          mint: esc.mint,
          index,
          sellerAmount: sRaw,
          buyerAmount: bRaw,
        })
        .rpc();
      setActionOk(`Resolved milestone ${index + 1} on ${address}`);
      await load();
    } catch (e) {
      setActionError(humanError(e));
    } finally {
      setBusyKey(null);
    }
  }

  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-xl font-bold">Arbiter console</h1>
        <WalletMultiButton />
      </div>
      <div className="card mt-4 p-4">
        <label className="label">Arbiter wallet (defaults to connected)</label>
        <div className="flex flex-col gap-2 sm:flex-row">
          <input
            className="input font-mono"
            value={input}
            onChange={(e) => setInput(e.target.value.trim())}
            placeholder={publicKey?.toBase58() ?? "Arbiter pubkey"}
          />
          <button className="btn btn-primary" disabled={!client || loading} onClick={load}>
            {loading ? "Loading…" : "Find disputes"}
          </button>
        </div>
        {error && <p className="mt-2 text-sm" style={{ color: "var(--danger)" }}>{error}</p>}
      </div>
      {actionError && <p className="mt-3 text-sm" style={{ color: "var(--danger)" }}>{actionError}</p>}
      {actionOk && <p className="mt-3 text-sm" style={{ color: "var(--success)" }}>{actionOk}</p>}
      <div className="mt-4 flex flex-col gap-3">
        {rows.map((r) => (
          <div key={r.address} className="card p-4">
            <Link className="font-mono text-sm underline" href={`/escrow/${r.address}`}>
              {r.address}
            </Link>
            <p className="mt-1 text-xs" style={{ color: "var(--muted)" }}>
              {r.disputed.length} disputed of {r.total} milestones
            </p>
            {r.disputed.map((i) => {
              const key = `${r.address}:${i}`;
              return (
                <div key={key} className="mt-2 flex flex-col gap-2 sm:flex-row sm:items-end">
                  <div>
                    <label className="label">Milestone #{i + 1} seller (base units)</label>
                    <input
                      className="input font-mono"
                      value={sellerAmt[key] ?? ""}
                      onChange={(e) => setSellerAmt((p) => ({ ...p, [key]: e.target.value.trim() }))}
                    />
                  </div>
                  <div>
                    <label className="label">Buyer (base units)</label>
                    <input
                      className="input font-mono"
                      value={buyerAmt[key] ?? ""}
                      onChange={(e) => setBuyerAmt((p) => ({ ...p, [key]: e.target.value.trim() }))}
                    />
                  </div>
                  <button className="btn" disabled={busyKey === key} onClick={() => resolve(r.address, i)}>
                    {busyKey === key ? "…" : "Resolve"}
                  </button>
                </div>
              );
            })}
          </div>
        ))}
        {!loading && rows.length === 0 && (
          <div className="card p-6 text-center text-sm" style={{ color: "var(--muted)" }}>
            No disputed escrows for this arbiter.
          </div>
        )}
      </div>
    </div>
  );
}
