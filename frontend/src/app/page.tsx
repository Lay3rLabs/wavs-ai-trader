import { Header } from "../components/Header";
import { Hero } from "../components/Hero";
import { VaultStats } from "../components/VaultStats";
import { DepositWithdraw } from "../components/DepositWithdraw";
import { PendingDeposits } from "../components/PendingDeposits";
import { TradeHistory } from "../components/TradeHistory";

export default function Home() {
  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950">
      <Header />
      <main>
        <Hero />
        <VaultStats />
        <DepositWithdraw />
        <PendingDeposits />
        <TradeHistory />
      </main>
      <footer className="border-t border-zinc-200 bg-white dark:border-zinc-800 dark:bg-black">
        <div className="container mx-auto px-4 py-8 sm:px-6 lg:px-8">
          <div className="flex flex-col items-center justify-between gap-4 sm:flex-row">
            <p className="text-sm text-zinc-600 dark:text-zinc-400">
              Powered by WAVS - Decentralized, Verifiable AI
            </p>
            <div className="flex gap-6">
              <a
                href="https://www.wavs.xyz"
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-zinc-600 transition-colors hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-50"
              >
                About WAVS
              </a>
              <a
                href="https://dorahacks.io/hackathon/hackmos-2025"
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-zinc-600 transition-colors hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-50"
              >
                Hackmos 2025
              </a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}
