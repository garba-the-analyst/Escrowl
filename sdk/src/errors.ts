// Error-code map (programs/escrowl/src/errors.rs, codes 6000-6024) plus
// human messages for the frontend. Anchor framework constraint errors
// (e.g. ConstraintHasOne, 2001) surface role attacks — see I2.
export const ERROR_CODES: Record<number, { name: string; msg: string }> = {
  6000: { name: "InvalidMilestoneCount", msg: "Milestones must be 1 to 8" },
  6001: { name: "ZeroMilestoneAmount", msg: "Each milestone amount must be above zero" },
  6002: { name: "TotalMismatch", msg: "Milestone amounts do not add up" },
  6003: { name: "MathOverflow", msg: "Arithmetic overflow — amount too large" },
  6004: { name: "DuplicateRole", msg: "Buyer, seller and arbiter must all be different wallets" },
  6005: { name: "MintNotAllowed", msg: "This token mint is not allowlisted" },
  6006: { name: "Token2022Rejected", msg: "Only classic SPL tokens are supported" },
  6007: { name: "InvalidMilestoneIndex", msg: "Milestone does not exist" },
  6008: { name: "OutOfOrderSubmit", msg: "Submit milestones in order, starting from the first" },
  6009: { name: "InvalidMilestoneState", msg: "Milestone is not in the right state for this action" },
  6010: { name: "InvalidEscrowState", msg: "Escrow is not in the right state for this action" },
  6011: { name: "WindowNotElapsed", msg: "Buyer review window has not ended yet" },
  6012: { name: "WindowElapsed", msg: "Buyer review window already ended" },
  6013: { name: "InvalidSplitSum", msg: "Dispute split must add up exactly to the milestone amount" },
  6014: { name: "FeeTooHigh", msg: "Protocol fee exceeds the 5% maximum" },
  6015: { name: "Paused", msg: "Protocol is paused for new escrows and funding" },
  6016: { name: "CancelNotAllowed", msg: "Cannot cancel — work has already started" },
  6017: { name: "NotAllTerminal", msg: "All milestones must be finished before closing" },
  6018: { name: "VaultBalanceMismatch", msg: "Vault balance does not match the expected amount" },
  6019: { name: "VaultNotEmpty", msg: "Vault must be empty for this action" },
  6020: { name: "Unauthorized", msg: "Your wallet is not authorized for this action" },
  6021: { name: "InvalidReviewWindow", msg: "Review window must be between 1 hour and 30 days" },
  6022: { name: "AllowlistUpdateInvalid", msg: "Mint is already allowlisted or the allowlist is full" },
  6023: { name: "SellerDeadlineNotElapsed", msg: "Seller inactivity deadline has not ended yet" },
  6024: { name: "DisputeNotExpired", msg: "Arbiter timeout has not ended yet" },
};

/** Anchor framework errors worth mapping in UI copy. */
export const FRAMEWORK_ERRORS: Record<string, string> = {
  ConstraintHasOne: "Your wallet is not authorized for this action",
  HasOneConstraintViolated: "Your wallet is not authorized for this action",
  ConstraintSigner: "This action requires a different signer",
  AccountNotInitialized: "Account does not exist (already closed?)",
};

function extractCode(e: unknown): number | null {
  const anyE = e as { error?: { errorCode?: { number?: number } } };
  return anyE?.error?.errorCode?.number ?? null;
}

/** Human message for any program/framework transaction error. */
export function humanError(e: unknown): string {
  const code = extractCode(e);
  if (code !== null && ERROR_CODES[code]) return ERROR_CODES[code].msg;
  const anyE = e as { error?: { errorMessage?: string }; message?: string };
  const msg: string = anyE?.error?.errorMessage ?? anyE?.message ?? String(e);
  for (const [key, friendly] of Object.entries(FRAMEWORK_ERRORS)) {
    if (msg.includes(key)) return friendly;
  }
  return msg;
}

/** True if the error is the program error with the given name. */
export function isError(e: unknown, name: string): boolean {
  const code = extractCode(e);
  if (code !== null) return ERROR_CODES[code]?.name === name;
  const msg = String((e as { message?: string })?.message ?? e);
  return msg.includes(name);
}
