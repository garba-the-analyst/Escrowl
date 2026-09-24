import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import { Providers } from "@/components/providers";
import "./globals.css";

export const metadata: Metadata = {
  title: "Escrowl — Milestone SPL Escrow",
  description: "Milestone-based, dispute-aware SPL escrow on Solana",
};

const PROGRAM_ID =
  process.env.NEXT_PUBLIC_PROGRAM_ID ?? "61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE";

export default function RootLayout({ children }: { children: ReactNode }) {
  const explorer = `https://explorer.solana.com/address/${PROGRAM_ID}?cluster=devnet`;
  return (
    <html lang="en">
      <body>
        <Providers>
          <header className="border-b" style={{ borderColor: "var(--border)" }}>
            <nav className="mx-auto flex max-w-5xl flex-wrap items-center gap-4 px-4 py-3">
              <Link href="/" className="text-lg font-bold">
                Escrowl
              </Link>
              <div className="flex gap-2 text-sm">
                <Link className="btn" href="/">
                  Dashboard
                </Link>
                <Link className="btn" href="/create">
                  Create
                </Link>
                <Link className="btn" href="/arbiter">
                  Arbiter
                </Link>
              </div>
            </nav>
          </header>
          <main className="mx-auto w-full max-w-5xl px-4 py-6">{children}</main>
          <footer
            className="mx-auto max-w-5xl px-4 py-6 text-xs"
            style={{ color: "var(--muted)" }}
          >
            <p className="break-all">Program: {PROGRAM_ID}</p>
            <a className="underline" href={explorer} target="_blank" rel="noreferrer">
              View program on Solana Explorer (devnet)
            </a>
          </footer>
        </Providers>
      </body>
    </html>
  );
}
