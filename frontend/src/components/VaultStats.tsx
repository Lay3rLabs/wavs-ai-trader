"use client";

import { useVault } from '../hooks/useVault';
import { useWallet } from '../contexts/WalletContext';
import { useTokenMetadata, formatTokenAmount } from '../hooks/useTokenMetadata';

function formatNumber(value: string): string {
  try {
    const num = parseFloat(value);
    if (num >= 1000000) {
      return `$${(num / 1000000).toFixed(2)}M`;
    } else if (num >= 1000) {
      return `$${(num / 1000).toFixed(2)}K`;
    }
    return `$${num.toFixed(2)}`;
  } catch {
    return '$0.00';
  }
}

function formatShares(shares: string): string {
  try {
    const num = parseFloat(shares);
    if (num >= 1000000000) {
      return `${(num / 1000000000).toFixed(2)}B`;
    } else if (num >= 1000000) {
      return `${(num / 1000000).toFixed(2)}M`;
    } else if (num >= 1000) {
      return `${(num / 1000).toFixed(2)}K`;
    }
    return num.toFixed(0);
  } catch {
    return '0';
  }
}

export function VaultStats() {
  const { vaultState, isLoading, userShares } = useVault();
  const { isConnected } = useWallet();

  const allDenoms = vaultState ? [
    ...vaultState.funds.map(c => c.denom),
    ...vaultState.total_pending_assets.map(c => c.denom),
  ] : [];
  const { metadata: tokenMetadata } = useTokenMetadata(allDenoms);

  if (isLoading) {
    return (
      <div className="border-b border-zinc-200 bg-white dark:border-zinc-800 dark:bg-black">
        <div className="container mx-auto px-4 py-8 sm:px-6 lg:px-8">
          <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="animate-pulse rounded-lg border border-zinc-200 bg-zinc-50 p-6 dark:border-zinc-800 dark:bg-zinc-900">
                <div className="h-4 w-20 rounded bg-zinc-300 dark:bg-zinc-700" />
                <div className="mt-2 h-8 w-32 rounded bg-zinc-300 dark:bg-zinc-700" />
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  const tvl = vaultState ? formatNumber(vaultState.tvl) : '$0.00';
  const assetsCount = vaultState?.funds.length || 0;
  const pendingCount = vaultState?.total_pending_assets.length || 0;

  return (
    <div className="border-b border-zinc-200 bg-white dark:border-zinc-800 dark:bg-black">
      <div className="container mx-auto px-4 py-8 sm:px-6 lg:px-8">
        <div className="mb-6 flex items-center justify-between">
          <h2 className="text-2xl font-semibold text-zinc-900 dark:text-zinc-50">
            Vault Overview
          </h2>
          {isConnected && (
            <div className="flex items-center gap-2 rounded-lg border border-blue-200 bg-blue-50 px-4 py-2 dark:border-blue-900 dark:bg-blue-950">
              <svg className="h-5 w-5 text-blue-600 dark:text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
              </svg>
              <div>
                <p className="text-xs font-medium text-blue-900 dark:text-blue-100">Your Shares</p>
                <p className="text-sm font-semibold text-blue-900 dark:text-blue-100">{formatShares(userShares)}</p>
              </div>
            </div>
          )}
        </div>

        <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
          <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-6 transition-colors hover:border-zinc-300 dark:border-zinc-800 dark:bg-zinc-900 dark:hover:border-zinc-700">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium text-zinc-600 dark:text-zinc-400">Total Value Locked</p>
              <svg className="h-5 w-5 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <p className="mt-2 text-3xl font-bold text-zinc-900 dark:text-zinc-50">{tvl}</p>
          </div>

          <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-6 transition-colors hover:border-zinc-300 dark:border-zinc-800 dark:bg-zinc-900 dark:hover:border-zinc-700">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium text-zinc-600 dark:text-zinc-400">Active Assets</p>
              <svg className="h-5 w-5 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
            </div>
            <p className="mt-2 text-3xl font-bold text-zinc-900 dark:text-zinc-50">{assetsCount}</p>
            <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">Token types</p>
          </div>

          <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-6 transition-colors hover:border-zinc-300 dark:border-zinc-800 dark:bg-zinc-900 dark:hover:border-zinc-700">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium text-zinc-600 dark:text-zinc-400">Pending Deposits</p>
              <svg className="h-5 w-5 text-yellow-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <p className="mt-2 text-3xl font-bold text-zinc-900 dark:text-zinc-50">{pendingCount}</p>
            <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">Awaiting price update</p>
          </div>

          <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-6 transition-colors hover:border-zinc-300 dark:border-zinc-800 dark:bg-zinc-900 dark:hover:border-zinc-700">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium text-zinc-600 dark:text-zinc-400">Current Prices</p>
              <svg className="h-5 w-5 text-purple-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
              </svg>
            </div>
            <p className="mt-2 text-3xl font-bold text-zinc-900 dark:text-zinc-50">
              {vaultState?.prices.length || 0}
            </p>
            <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">Active price feeds</p>
          </div>
        </div>

        {vaultState && vaultState.funds.length > 0 && (
          <div className="mt-6">
            <h3 className="mb-3 text-sm font-medium text-zinc-600 dark:text-zinc-400">Holdings</h3>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {vaultState.funds.map((coin, idx) => {
                const price = vaultState.prices.find(p => p.denom === coin.denom);
                const meta = tokenMetadata.get(coin.denom);
                const displayAmount = formatTokenAmount(coin.amount, meta?.decimals || 6);
                const displaySymbol = meta?.symbol || coin.denom.replace('u', '').toUpperCase();

                return (
                  <div
                    key={idx}
                    className="flex items-center justify-between rounded-lg border border-zinc-200 bg-white p-4 dark:border-zinc-800 dark:bg-zinc-950"
                  >
                    <div>
                      <p className="text-sm font-medium text-zinc-900 dark:text-zinc-50">
                        {displayAmount} {displaySymbol}
                      </p>
                      {price && (
                        <p className="text-xs text-zinc-500 dark:text-zinc-500">
                          @ ${parseFloat(price.price_usd).toFixed(2)}
                        </p>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
