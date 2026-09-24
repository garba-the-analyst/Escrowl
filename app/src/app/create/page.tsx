"use client";
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

import { useMemo, useState } from "react";
import Link from "next/link";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { humanError } from "@escrowl/sdk/src/errors";
import { useEscrowlClient } from "@/lib/client";
import { parseAmount } from "@/lib/format";

const DEVNET_USDC = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDnc";
const REVIEW_OPTIONS = [
  { label: "1 day", secs: "86400" },
  { label: "3 days", secs: "259200" },
  { label: "7 days", secs: "604800" },
];
const SELLER_DEADLINE_OPTIONS = [
  { label: "1 day", secs: "86400" },
  { label: "7 days", secs: "604800" },
  { label: "30 days", secs: "2592000" },
];
const ARBITER_TIMEOUT_OPTIONS = [
  { label: "7 days", secs: "604800" },
  { label: "14 days", secs: "1209600" },
  { label: "30 days", secs: "2592000" },
];

export default function CreatePage() {
  const { publicKey } = useWallet();
  const client = useEscrowlClient();
  const [seller, setSeller] = useState("");
  const [arbiter, setArbiter] = useState("");
  const [mint, setMint] = useState(DEVNET_USDC);
  const [amounts, setAmounts] = useState<string[]>(["100"]);
  const [reviewSecs, setReviewSecs] = useState(REVIEW_OPTIONS[1].secs);
  const [sellerDeadlineSecs, setSellerDeadlineSecs] = useState(SELLER_DEADLINE_OPTIONS[1].secs);
  const [arbiterTimeoutSecs, setArbiterTimeoutSecs] = useState(ARBITER_TIMEOUT_OPTIONS[2].secs);
  const [busy, setBusy] = useState(false);
  const [step, setStep] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [created, setCreated] = useState<string | null>(null);

  const validation = useMemo(() => {
    try {
      if (!publicKey) return "Connect a wallet first";
      const s = new PublicKey(seller);
      const a = new PublicKey(arbiter);
      const m = new PublicKey(mint);
      void m;
      if (amounts.length < 1 || amounts.length > 8) return "Milestones must be 1 to 8";
      for (const x of amounts) parseAmount(x, 6);
      const w = BigInt(reviewSecs);
      if (w < 3600n || w > 2592000n) return "Review window must be 1 hour to 30 days";
      const d = BigInt(sellerDeadlineSecs);
      if (d < 86400n || d > 7776000n) return "Seller deadline must be 1 to 90 days";
      const t = BigInt(arbiterTimeoutSecs);
      if (t < 604800n || t > 7776000n) return "Arbiter timeout must be 7 to 90 days";
      const buyer58 = publicKey.toBase58();
      if (s.toBase58() === buyer58 || a.toBase58() === buyer58 || s.toBase58() === a.toBase58())
        return "Buyer, seller and arbiter must all be different wallets";
      return null;
    } catch (e) {
      return e instanceof Error ? e.message : "Invalid input";
    }
  }, [publicKey, seller, arbiter, mint, amounts, reviewSecs, sellerDeadlineSecs, arbiterTimeoutSecs]);

  function setAmount(i: number, v: string) {
    setAmounts((prev) => prev.map((x, j) => (j === i ? v : x)));
  }

  async function onSubmit() {
    if (!client || !publicKey || validation) return;
    setBusy(true);
    setError(null);
    setCreated(null);
    try {
      const sellerPk = new PublicKey(seller);
      const arbiterPk = new PublicKey(arbiter);
      const mintPk = new PublicKey(mint);
      const escrowId = new BN(Date.now().toString());
      const milestoneAmounts = amounts.map((x) => new BN(parseAmount(x, 6)));

      setStep("Creating escrow…");
      const escrow = client.escrowPda(publicKey, sellerPk, escrowId);
      await client
        .createEscrow({
          buyer: publicKey,
          seller: sellerPk,
          arbiter: arbiterPk,
          mint: mintPk,
          escrowId,
          milestoneAmounts,
          reviewWindowSecs: new BN(reviewSecs),
          sellerDeadlineSecs: new BN(sellerDeadlineSecs),
          arbiterTimeoutSecs: new BN(arbiterTimeoutSecs),
        })
        .rpc();

      setStep("Funding escrow…");
      const buyerAta = getAssociatedTokenAddressSync(mintPk, publicKey);
      await client.fundEscrow(publicKey, escrow, buyerAta, mintPk).rpc();

      setStep(null);
      setCreated(escrow.toBase58());
    } catch (e) {
      setError(humanError(e));
      setStep(null);
    } finally {
      setBusy(false);
    }
  }

  if (!publicKey) {
    return (
      <div className="card p-6 text-center">
        <h1 className="text-xl font-bold">Create escrow</h1>
        <p className="mt-2 text-sm" style={{ color: "var(--muted)" }}>
          Connect your wallet to create a milestone escrow.
        </p>
        <div className="mt-4 flex justify-center">
          <WalletMultiButton />
        </div>
      </div>
    );
  }

  return (
    <div className="card mx-auto max-w-2xl p-6">
      <h1 className="text-xl font-bold">Create escrow</h1>
      <p className="mt-1 text-xs" style={{ color: "var(--muted)" }}>
        Mint defaults to a devnet USDC placeholder ({DEVNET_USDC}). Only
        allowlisted SPL classic mints are accepted on-chain.
      </p>
      <div className="mt-4 flex flex-col gap-3">
        <div>
          <label className="label">Seller wallet</label>
          <input className="input font-mono" value={seller} onChange={(e) => setSeller(e.target.value.trim())} placeholder="Seller pubkey" />
        </div>
        <div>
          <label className="label">Arbiter wallet</label>
          <input className="input font-mono" value={arbiter} onChange={(e) => setArbiter(e.target.value.trim())} placeholder="Arbiter pubkey" />
        </div>
        <div>
          <label className="label">Mint</label>
          <input className="input font-mono" value={mint} onChange={(e) => setMint(e.target.value.trim())} placeholder="SPL mint" />
        </div>
        <div>
          <label className="label">Milestones (1–8, amounts in tokens)</label>
          {amounts.map((a, i) => (
            <div key={i} className="mb-2 flex gap-2">
              <input className="input font-mono" value={a} onChange={(e) => setAmount(i, e.target.value.trim())} placeholder={`Milestone ${i + 1}`} />
              {amounts.length > 1 && (
                <button className="btn" disabled={busy} onClick={() => setAmounts((p) => p.filter((_, j) => j !== i))}>
                  Remove
                </button>
              )}
            </div>
          ))}
          {amounts.length < 8 && (
            <button className="btn mt-1" disabled={busy} onClick={() => setAmounts((p) => [...p, "10"])}>
              Add milestone
            </button>
          )}
        </div>
        <div>
          <label className="label">Review window</label>
          <select className="input" value={reviewSecs} onChange={(e) => setReviewSecs(e.target.value)}>
            {REVIEW_OPTIONS.map((o) => (
              <option key={o.secs} value={o.secs}>
                {o.label}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="label">Seller deadline (buyer may reclaim a stale milestone after this)</label>
          <select className="input" value={sellerDeadlineSecs} onChange={(e) => setSellerDeadlineSecs(e.target.value)}>
            {SELLER_DEADLINE_OPTIONS.map((o) => (
              <option key={o.secs} value={o.secs}>
                {o.label}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="label">Arbiter timeout (disputes unlock 50/50 after this)</label>
          <select className="input" value={arbiterTimeoutSecs} onChange={(e) => setArbiterTimeoutSecs(e.target.value)}>
            {ARBITER_TIMEOUT_OPTIONS.map((o) => (
              <option key={o.secs} value={o.secs}>
                {o.label}
              </option>
            ))}
          </select>
        </div>
        {validation && <p className="text-xs" style={{ color: "var(--warning)" }}>{validation}</p>}
        {step && <p className="text-sm">{step}</p>}
        {error && <p className="text-sm" style={{ color: "var(--danger)" }}>{error}</p>}
        {created && (
          <p className="text-sm" style={{ color: "var(--success)" }}>
            Escrow created and funded.{" "}
            <Link className="underline" href={`/escrow/${created}`}>
              View detail
            </Link>
          </p>
        )}
        <button className="btn btn-primary" disabled={busy || !!validation} onClick={onSubmit}>
          {busy ? "Submitting…" : "Create + Fund"}
        </button>
      </div>
    </div>
  );
}
