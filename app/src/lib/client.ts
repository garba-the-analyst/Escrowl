"use client";
import { AnchorProvider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

import { useMemo } from "react";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";
import { EscrowlClient } from "@escrowl/sdk/src";

/** Build an EscrowlClient from the connected wallet. Null when disconnected. */
export function useEscrowlClient(): EscrowlClient | null {
  const wallet = useAnchorWallet();
  const { connection } = useConnection();

  return useMemo(() => {
    if (!wallet) return null;
    const provider = new AnchorProvider(connection, wallet, {
      commitment: "confirmed",
    });
    const programIdRaw = process.env.NEXT_PUBLIC_PROGRAM_ID;
    const programId = programIdRaw ? new PublicKey(programIdRaw) : undefined;
    return new EscrowlClient(provider, programId);
  }, [wallet, connection]);
}
