"use client";

import { useVault } from '../hooks/useVault';
import { useTokenMetadata, formatTokenAmount } from '../hooks/useTokenMetadata';

export function PendingDeposits() {
  const { pendingDeposits, isLoading } = useVault();

  const allDenoms = pendingDeposits.flatMap(d => d.coins.map(c => c.denom));
  const { metadata: tokenMetadata } = useTokenMetadata(allDenoms);

  if (isLoading) {
    return (
      <div className="bg-zinc-50 dark:bg-zinc-950">
        <div className="container mx-auto px-4 py-12 sm:px-6 lg:px-8">
          <div className="mx-auto max-w-4xl">
            <div className="animate-pulse rounded-lg border border-zinc-200 bg-white p-8 dark:border-zinc-800 dark:bg-black">
              <div className="h-6 w-48 rounded bg-zinc-300 dark:bg-zinc-700" />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (pendingDeposits.length === 0) {
    return null;
  }

  return (
    <div className="bg-zinc-50 dark:bg-zinc-950">
      <div className="container mx-auto px-4 py-12 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-4xl">
          <h2 className="mb-6 text-2xl font-semibold text-zinc-900 dark:text-zinc-50">
            Pending Deposits
          </h2>

          <div className="space-y-4">
            {pendingDeposits.map((deposit) => {
              const displayCoins = deposit.coins
                .map((coin) => {
                  const meta = tokenMetadata.get(coin.denom);
                  const amount = formatTokenAmount(coin.amount, meta?.decimals || 6);
                  const symbol = meta?.symbol || coin.denom.replace('u', '').toUpperCase();
                  return `${amount} ${symbol}`;
                })
                .join(' + ');

              return (
                <div
                  key={deposit.id}
                  className="rounded-lg border border-zinc-200 bg-white p-6 dark:border-zinc-800 dark:bg-black"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <h3 className="text-lg font-medium text-zinc-900 dark:text-zinc-50">
                          Deposit #{deposit.id}
                        </h3>
                        <span className="inline-flex items-center rounded-full border border-yellow-200 bg-yellow-50 px-2.5 py-0.5 text-xs font-medium text-yellow-800 dark:border-yellow-900 dark:bg-yellow-950 dark:text-yellow-200">
                          Pending
                        </span>
                      </div>
                      <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
                        {displayCoins}
                      </p>
                      <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">
                        From: {deposit.user.slice(0, 20)}...{deposit.user.slice(-10)}
                      </p>
                    </div>
                    <div className="ml-4">
                      <svg className="h-8 w-8 text-yellow-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                      </svg>
                    </div>
                  </div>
                  <div className="mt-4 rounded-lg border border-zinc-200 bg-zinc-50 p-3 dark:border-zinc-800 dark:bg-zinc-900">
                    <p className="text-xs text-zinc-600 dark:text-zinc-400">
                      This deposit is waiting for the next price update from WAVS AI agents.
                      Once prices are updated, shares will be issued automatically.
                    </p>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
