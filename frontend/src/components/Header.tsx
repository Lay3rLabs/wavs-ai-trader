"use client";

import { useWallet } from '../contexts/WalletContext';

export function Header() {
  const { address, isConnected, isConnecting, connect, disconnect } = useWallet();

  const shortenAddress = (addr: string) => {
    return `${addr.slice(0, 10)}...${addr.slice(-6)}`;
  };

  return (
    <header className="sticky top-0 z-50 w-full border-b border-zinc-200 bg-white/80 backdrop-blur-sm dark:border-zinc-800 dark:bg-black/80">
      <div className="container mx-auto flex h-16 items-center justify-between px-4 sm:px-6 lg:px-8">
        <div className="flex items-center gap-2">
          <span className="text-xl font-semibold text-zinc-900 dark:text-zinc-50">
            WAVS AI Trader
          </span>
        </div>

        <div>
          {isConnected ? (
            <div className="flex items-center gap-4">
              <div className="hidden sm:flex items-center gap-2 rounded-full border border-zinc-200 bg-zinc-50 px-4 py-2 text-sm dark:border-zinc-800 dark:bg-zinc-900">
                <div className="h-2 w-2 rounded-full bg-green-500" />
                <span className="text-zinc-700 dark:text-zinc-300">
                  {shortenAddress(address!)}
                </span>
              </div>
              <button
                onClick={disconnect}
                className="rounded-lg border border-zinc-200 px-4 py-2 text-sm font-medium text-zinc-700 transition-colors hover:bg-zinc-50 dark:border-zinc-800 dark:text-zinc-300 dark:hover:bg-zinc-900"
              >
                Disconnect
              </button>
            </div>
          ) : (
            <button
              onClick={connect}
              disabled={isConnecting}
              className="rounded-lg bg-zinc-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-zinc-800 disabled:opacity-50 dark:bg-zinc-50 dark:text-zinc-900 dark:hover:bg-zinc-200"
            >
              {isConnecting ? 'Connecting...' : 'Connect Wallet'}
            </button>
          )}
        </div>
      </div>
    </header>
  );
}
