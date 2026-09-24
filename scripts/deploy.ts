// Deploy to devnet + post-deploy checklist.
// Run: anchor deploy --provider.cluster devnet  (or: ts-node scripts/deploy.ts)
// Then follow the printed checklist: the placeholder program ID 111... must be
// replaced everywhere with the real deployed address.
import { execSync } from "child_process";
import * as fs from "fs";
import * as path from "path";

const ROOT = path.join(__dirname, "..");

function main() {
  console.log("=== Escrowl devnet deploy ===\n");

  execSync("anchor deploy --provider.cluster devnet", {
    stdio: "inherit",
    cwd: ROOT,
  });

  // Read back the program ID anchor deployed (from Anchor.toml).
  const toml = fs.readFileSync(path.join(ROOT, "Anchor.toml"), "utf8");
  const match = toml.match(/\[programs\.devnet\]\s*escrowl\s*=\s*"([^"]+)"/);
  const programId = match?.[1] ?? "<unknown — check `anchor keys list`>";
  console.log(`\nDeployed program: ${programId}`);
  console.log(
    `Explorer: https://explorer.solana.com/address/${programId}?cluster=devnet\n`
  );

  console.log("Post-deploy checklist (do all before demo/bounty update):");
  console.log("  1. `anchor keys sync` to pin the keypair address everywhere.");
  console.log("  2. Regenerate IDL: `anchor build` then diff target/idl/escrowl.json");
  console.log("     against sdk/idl/escrowl.json (see sdk/idl/README.md).");
  console.log('  3. Verifiable build: `solana-verify build --library-name escrowl`.');
  console.log("  4. Update docs/DEPLOYMENT.md with program ID + tx signatures.");
  console.log("  5. Set app/.env.local NEXT_PUBLIC_PROGRAM_ID=<id>.");
  console.log("  6. Run seed: `yarn seed-devnet` (scripts/seed-devnet.ts).");
  console.log("  7. Update Superteam Earn bounty submission with repo + program ID.");
}

main();
