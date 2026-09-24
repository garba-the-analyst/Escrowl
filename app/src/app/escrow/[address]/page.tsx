"use client";
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useParams } from "next/navigation";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import type { EscrowAccount } from "@escrowl/sdk/src/types";
import { escrowStatus, milestoneStatus } from "@escrowl/sdk/src/types";
import { humanError } from "@escrowl/sdk/src/errors";
import { useEscrowlClient } from "@/lib/client";
import { formatAmount, formatTimestamp, shortAddress } from "@/lib/format";
import { StatusBadge } from "@/components/status-badge";
import {
  MilestoneTimeline,
  type ActionPermission,
  type MilestoneAction,
} from "@/components/milestone-timeline";

interface FeedEvent {
  name: string;
  at: string;
}

export default function EscrowDetail() {
  const params = useParams<{ address: string }>();
  const address = params.address;
  const { publicKey } = useWallet();
  const { connection } = useConnection();
  const client = useEscrowlClient();

  const [escrow, setEscrow] = useState<EscrowAccount | null>(null);
  const [vaultBalance, setVaultBalance] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingIndex, setPendingIndex] = useState<number | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionOk, setActionOk] = useState<string | null>(null);
  const [evidence, setEvidence] = useState("");
  const [splitSeller, setSplitSeller] = useState("");
  const [splitBuyer, setSplitBuyer] = useState("");
  const [escrowBusy, setEscrowBusy] = useState(false);
  const [feed, setFeed] = useState<FeedEvent[]>([]);

  const refresh = useCallback(async () => {
    if (!client || !address) return;
    setLoading(true);
    setError(null);
    try {
      const pk = new PublicKey(address);
      const acc = await client.getEscrow(pk);
      setEscrow(acc);
      try {
        const vault = client.vaultPda(pk);
        const bal = await connection.getTokenAccountBalance(vault).catch(() => null);
        setVaultBalance(bal?.value.amount ?? null);
      } catch {
        setVaultBalance(null);
      }
    } catch (e) {
      setError(humanError(e));
    } finally {
      setLoading(false);
    }
  }, [client, address, connection]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Event feed: subscribe to all program events, filter by escrow address.
  useEffect(() => {
    if (!client || !address) return;
    let ids: number[] = [];
    try {
      ids = client.onAllEvents((name, data) => {
        const s = JSON.stringify(data ?? {});
        if (s.includes(address)) {
          setFeed((f) => [{ name, at: new Date().toLocaleString() }, ...f].slice(0, 30));
        }
      });
    } catch {
      ids = [];
    }
    const snapshot = [...ids];
    return () => {
      if (snapshot.length > 0) client.removeListeners(snapshot).catch(() => undefined);
    };
  }, [client, address]);

  const roles = useMemo(() => {
    if (!escrow || !publicKey)
      return { isBuyer: false, isSeller: false, isArbiter: false };
    const me = publicKey.toBase58();
    return {
      isBuyer: escrow.buyer.toBase58() === me,
      isSeller: escrow.seller.toBase58() === me,
      isArbiter: escrow.arbiter.toBase58() === me,
    };
  }, [escrow, publicKey]);

  const permissions = useMemo(() => {
    const perms: Record<MilestoneAction, (index: number) => ActionPermission> = {
      submit: (i) => {
        if (!roles.isSeller) return { allowed: false, reason: "Only the seller can submit" };
        if (!escrow) return { allowed: false, reason: "Loading" };
        const st = milestoneStatus(escrow.milestones[i]);
        if (st !== "pending") return { allowed: false, reason: `Milestone is ${st}` };
        if (i > 0 && milestoneStatus(escrow.milestones[i - 1]) === "pending")
          return { allowed: false, reason: "Submit milestones in order" };
        return { allowed: true };
      },
      approve: (i) => {
        if (!roles.isBuyer) return { allowed: false, reason: "Only the buyer can approve" };
        if (!escrow) return { allowed: false, reason: "Loading" };
        const st = milestoneStatus(escrow.milestones[i]);
        if (st !== "submitted") return { allowed: false, reason: `Milestone is ${st}` };
        return { allowed: true };
      },
      claim: (i) => {
        if (!roles.isSeller) return { allowed: false, reason: "Only the seller can claim" };
        if (!escrow) return { allowed: false, reason: "Loading" };
        const st = milestoneStatus(escrow.milestones[i]);
        if (st !== "submitted") return { allowed: false, reason: `Milestone is ${st}` };
        return { allowed: true };
      },
      dispute: (i) => {
        if (!roles.isBuyer) return { allowed: false, reason: "Only the buyer can dispute" };
        if (!escrow) return { allowed: false, reason: "Loading" };
        const st = milestoneStatus(escrow.milestones[i]);
        if (st !== "submitted") return { allowed: false, reason: `Milestone is ${st}` };
        return { allowed: true };
      },
      resolve: (i) => {
        if (!roles.isArbiter) return { allowed: false, reason: "Only the arbiter can resolve" };
        if (!escrow) return { allowed: false, reason: "Loading" };
        const st = milestoneStatus(escrow.milestones[i]);
        if (st !== "disputed") return { allowed: false, reason: `Milestone is ${st}` };
        return { allowed: true };
      },
      reclaim: (i) => {
        if (!roles.isBuyer) return { allowed: false, reason: "Only the buyer can reclaim" };
        if (!escrow) return { allowed: false, reason: "Loading" };
        const st = milestoneStatus(escrow.milestones[i]);
        if (st !== "pending") return { allowed: false, reason: `Milestone is ${st}` };
        for (let j = 0; j < i; j++) {
          if (milestoneStatus(escrow.milestones[j]) === "pending")
            return { allowed: false, reason: "Earlier milestones still active" };
          if (!["released", "resolved", "refunded", "cancelled"].includes(milestoneStatus(escrow.milestones[j])))
            return { allowed: false, reason: "Earlier milestone not terminal" };
        }
        const activeSince =
          i === 0
            ? Number(escrow.fundedAt.toString())
            : Number(escrow.milestones[i - 1].terminalAt.toString());
        if (activeSince <= 0) return { allowed: false, reason: "Not active yet" };
        const nowSec = Math.floor(Date.now() / 1000);
        const deadline = activeSince + Number(escrow.sellerDeadlineSecs.toString());
        if (nowSec < deadline) return { allowed: false, reason: "Seller deadline not elapsed" };
        return { allowed: true };
      },
      expire: (i) => {
        if (!roles.isBuyer && !roles.isSeller)
          return { allowed: false, reason: "Only buyer or seller can expire" };
        if (!escrow) return { allowed: false, reason: "Loading" };
        const st = milestoneStatus(escrow.milestones[i]);
        if (st !== "disputed") return { allowed: false, reason: `Milestone is ${st}` };
        const raised = Number(escrow.milestones[i].disputedAt.toString());
        const nowSec = Math.floor(Date.now() / 1000);
        if (nowSec < raised + Number(escrow.arbiterTimeoutSecs.toString()))
          return { allowed: false, reason: "Arbiter timeout not elapsed" };
        return { allowed: true };
      },
    };
    return perms;
  }, [escrow, roles]);

  async function runAction(action: MilestoneAction, index: number) {
    if (!client || !publicKey || !escrow || !address) return;
    setPendingIndex(index);
    setActionError(null);
    setActionOk(null);
    try {
      const escrowPk = new PublicKey(address);
      const sellerAta = getAssociatedTokenAddressSync(escrow.mint, escrow.seller);
      const buyerAta = getAssociatedTokenAddressSync(escrow.mint, escrow.buyer);
      const treasuryAta = getAssociatedTokenAddressSync(escrow.mint, escrow.treasurySnapshot);
      if (action === "submit") {
        const raw = evidence.trim() || `milestone-${index}`;
        const bytes = Array.from(new TextEncoder().encode(raw).subarray(0, 32));
        while (bytes.length < 32) bytes.push(0);
        await client.submitMilestone(publicKey, escrowPk, index, bytes).rpc();
        setActionOk(`Milestone ${index + 1} submitted`);
      } else if (action === "approve") {
        await client.approveMilestone(publicKey, escrowPk, sellerAta, treasuryAta, escrow.mint, index).rpc();
        setActionOk(`Milestone ${index + 1} approved`);
      } else if (action === "claim") {
        await client.claimAfterTimeout(publicKey, escrowPk, sellerAta, treasuryAta, escrow.mint, index).rpc();
        setActionOk(`Milestone ${index + 1} claimed after timeout`);
      } else if (action === "dispute") {
        await client.raiseDispute(publicKey, escrowPk, index).rpc();
        setActionOk(`Dispute raised on milestone ${index + 1}`);
      } else if (action === "resolve") {
        const ms = escrow.milestones[index];
        const total = BigInt(ms.amount.toString());
        if (!splitSeller.trim() || !splitBuyer.trim())
          throw new Error("Enter seller and buyer split amounts (base units)");
        if (BigInt(splitSeller.trim()) + BigInt(splitBuyer.trim()) !== total)
          throw new Error("Split must add up exactly to the milestone amount");
        await client
          .resolveDispute({
            arbiter: publicKey,
            escrow: escrowPk,
            sellerAta,
            buyerAta,
            treasuryAta,
            mint: escrow.mint,
            index,
            sellerAmount: splitSeller.trim(),
            buyerAmount: splitBuyer.trim(),
          })
          .rpc();
        setActionOk(`Milestone ${index + 1} dispute resolved`);
      } else if (action === "reclaim") {
        const buyerAta = getAssociatedTokenAddressSync(escrow.mint, escrow.buyer);
        await client.reclaimStaleMilestone(publicKey, escrowPk, buyerAta, escrow.mint, index).rpc();
        setActionOk(`Stale milestones from #${index + 1} reclaimed`);
      } else if (action === "expire") {
        const buyerAta = getAssociatedTokenAddressSync(escrow.mint, escrow.buyer);
        await client
          .expireDispute(publicKey, escrow.buyer, escrow.seller, escrowPk, sellerAta, buyerAta, treasuryAta, escrow.mint, index)
          .rpc();
        setActionOk(`Dispute on milestone ${index + 1} expired 50/50`);
      }
      await refresh();
    } catch (e) {
      setActionError(humanError(e));
    } finally {
      setPendingIndex(null);
    }
  }

  async function cancelOrClose(kind: "cancel" | "close") {
    if (!client || !publicKey || !escrow || !address) return;
    setEscrowBusy(true);
    setActionError(null);
    setActionOk(null);
    try {
      const escrowPk = new PublicKey(address);
      if (kind === "cancel") {
        const buyerAta = getAssociatedTokenAddressSync(escrow.mint, escrow.buyer);
        await client.cancelEscrow(publicKey, escrowPk, buyerAta, escrow.mint).rpc();
        setActionOk("Escrow cancelled and refunded");
      } else {
        await client.closeEscrow(publicKey, escrowPk).rpc();
        setActionOk("Escrow closed");
      }
      await refresh();
    } catch (e) {
      setActionError(humanError(e));
    } finally {
      setEscrowBusy(false);
    }
  }

  if (loading) return <p className="text-sm">Loading escrow…</p>;
  if (error) return <p className="text-sm" style={{ color: "var(--danger)" }}>{error}</p>;
  if (!escrow) return <p className="text-sm">Escrow not found.</p>;

  const st = escrowStatus(escrow);
  const canCancel =
    roles.isBuyer &&
    (st === "created" || st === "funded") &&
    !escrow.milestones.some((m) => milestoneStatus(m) !== "pending");
  const allTerminal =
    escrow.milestones.length > 0 &&
    escrow.milestones.every((m) =>
      ["released", "resolved", "refunded", "cancelled"].includes(milestoneStatus(m))
    );

  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="font-mono text-lg font-bold">{shortAddress(address)}</h1>
        <WalletMultiButton />
      </div>
      <div className="card mt-4 p-4 text-sm">
        <div className="flex flex-wrap items-center gap-2">
          <StatusBadge status={st} />
          <span>Total {formatAmount(escrow.totalAmount)}</span>
          <span style={{ color: "var(--muted)" }}>
            Released {formatAmount(escrow.releasedAmount)} · Refunded{" "}
            {formatAmount(escrow.refundedAmount)}
          </span>
        </div>
        <div className="mt-2 grid gap-1 font-mono text-xs" style={{ color: "var(--muted)" }}>
          <p>Buyer: {escrow.buyer.toBase58()}</p>
          <p>Seller: {escrow.seller.toBase58()}</p>
          <p>Arbiter: {escrow.arbiter.toBase58()}</p>
          <p>Mint: {escrow.mint.toBase58()}</p>
          <p>Vault: {vaultBalance ?? "—"} base units · created {formatTimestamp(escrow.createdAt)}</p>
          <p>Review window: {Number(new BN(escrow.reviewWindowSecs.toString()).toString())}s</p>
          <p>Seller deadline: {Number(new BN(escrow.sellerDeadlineSecs.toString()).toString())}s · Arbiter timeout: {Number(new BN(escrow.arbiterTimeoutSecs.toString()).toString())}s</p>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          <button
            className="btn btn-danger"
            disabled={!canCancel || escrowBusy}
            title={canCancel ? "Cancel and refund" : "Cancel only if no milestone was ever submitted"}
            onClick={() => cancelOrClose("cancel")}
          >
            Cancel
          </button>
          <button
            className="btn"
            disabled={!(roles.isBuyer && allTerminal) || escrowBusy}
            title={roles.isBuyer ? (allTerminal ? "Close escrow" : "All milestones must be finished") : "Only the buyer can close"}
            onClick={() => cancelOrClose("close")}
          >
            Close
          </button>
        </div>
      </div>

      {roles.isSeller && (
        <div className="card mt-3 p-4">
          <label className="label">Evidence (text or hash, stored as 32 bytes)</label>
          <input className="input" value={evidence} onChange={(e) => setEvidence(e.target.value)} placeholder="ipfs://… or description" />
        </div>
      )}
      {roles.isArbiter && (
        <div className="card mt-3 grid gap-2 p-4 sm:grid-cols-2">
          <div>
            <label className="label">Resolve split: seller (base units)</label>
            <input className="input font-mono" value={splitSeller} onChange={(e) => setSplitSeller(e.target.value.trim())} />
          </div>
          <div>
            <label className="label">Resolve split: buyer (base units)</label>
            <input className="input font-mono" value={splitBuyer} onChange={(e) => setSplitBuyer(e.target.value.trim())} />
          </div>
          <p className="text-xs sm:col-span-2" style={{ color: "var(--muted)" }}>
            Seller + buyer must sum exactly to the milestone amount.
          </p>
        </div>
      )}

      {actionError && <p className="mt-3 text-sm" style={{ color: "var(--danger)" }}>{actionError}</p>}
      {actionOk && <p className="mt-3 text-sm" style={{ color: "var(--success)" }}>{actionOk}</p>}

      <h2 className="mb-2 mt-6 font-bold">Milestones</h2>
      <MilestoneTimeline
        milestones={escrow.milestones}
        reviewWindowSecs={new BN(escrow.reviewWindowSecs.toString())}
        permissions={permissions}
        pendingIndex={pendingIndex}
        onAction={runAction}
      />

      <h2 className="mb-2 mt-6 font-bold">Events</h2>
      <div className="card p-4 text-xs" style={{ color: "var(--muted)" }}>
        {feed.length === 0 ? (
          <p>No events for this escrow yet in this session.</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {feed.map((f, i) => (
              <li key={i} className="font-mono">
                {f.at} — {f.name}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
