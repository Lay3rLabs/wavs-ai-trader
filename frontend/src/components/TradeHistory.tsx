"use client";

import { useTransactionHistory } from '../hooks/useTransactionHistory';
import { useTokenMetadata, formatTokenAmount } from '../hooks/useTokenMetadata';

export function TradeHistory() {
  const { events, isLoading, error } = useTransactionHistory();

  // Extract all denoms from events for metadata lookup
  const allDenoms = events.flatMap(event => {
    const denoms: string[] = [];
    Object.keys(event.data).forEach(key => {
      if (key.startsWith('u') || key.includes('ibc/') || key.includes('factory/')) {
        denoms.push(event.data[key]);
      }
    });
    return denoms;
  });
  const { metadata: tokenMetadata } = useTokenMetadata(allDenoms);

  const formatTimestamp = (timestamp: string) => {
    try {
      return new Date(timestamp).toLocaleString('en-US', {
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return 'Unknown';
    }
  };

  const getEventIcon = (type: string) => {
    switch (type) {
      case 'deposit':
        return (
          <svg className="h-5 w-5 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
        );
      case 'deposit_processed':
        return (
          <svg className="h-5 w-5 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        );
      case 'withdraw':
        return (
          <svg className="h-5 w-5 text-orange-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 12H4" />
          </svg>
        );
      case 'rebalancing':
        return (
          <svg className="h-5 w-5 text-purple-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" />
          </svg>
        );
      case 'price_update':
        return (
          <svg className="h-5 w-5 text-yellow-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
          </svg>
        );
      default:
        return (
          <svg className="h-5 w-5 text-zinc-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        );
    }
  };

  const getEventTitle = (event: typeof events[0]) => {
    switch (event.type) {
      case 'deposit':
        return 'Deposit Created';
      case 'deposit_processed':
        return 'Deposit Processed';
      case 'withdraw':
        return 'Withdrawal';
      case 'rebalancing':
        return 'AI Rebalancing';
      case 'price_update':
        return 'Price Updated';
      default:
        return 'Event';
    }
  };

  const getEventDescription = (event: typeof events[0]) => {
    const data = event.data;

    switch (event.type) {
      case 'deposit':
        if (data.user) {
          return `${data.user.slice(0, 10)}...${data.user.slice(-6)} deposited`;
        }
        return 'User deposited tokens';

      case 'deposit_processed':
        if (data.shares_issued && data.value_usd) {
          return `Issued ${parseFloat(data.shares_issued).toLocaleString()} shares ($${parseFloat(data.value_usd).toFixed(2)})`;
        }
        return 'Deposit converted to shares';

      case 'withdraw':
        if (data.shares && data.value_usd) {
          return `Withdrew ${parseFloat(data.shares).toLocaleString()} shares ($${parseFloat(data.value_usd).toFixed(2)})`;
        }
        return 'User withdrew funds';

      case 'rebalancing':
        if (data.swap_count) {
          return `AI executed ${data.swap_count} swap${parseInt(data.swap_count) > 1 ? 's' : ''}`;
        }
        return 'AI rebalanced portfolio';

      case 'price_update':
        if (data.denom && data.price_usd) {
          const meta = tokenMetadata.get(data.denom);
          const symbol = meta?.symbol || data.denom.replace('u', '').toUpperCase();
          return `${symbol} → $${parseFloat(data.price_usd).toFixed(4)}`;
        }
        return 'Price feed updated';

      default:
        return 'Transaction event';
    }
  };

  return (
    <div className="bg-zinc-50 dark:bg-zinc-950">
      <div className="container mx-auto px-4 py-12 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-4xl">
          <div className="mb-6 flex items-center justify-between">
            <h2 className="text-2xl font-semibold text-zinc-900 dark:text-zinc-50">
              Activity History
            </h2>
            {events.length > 0 && (
              <span className="text-sm text-zinc-500 dark:text-zinc-500">
                {events.length} event{events.length !== 1 ? 's' : ''}
              </span>
            )}
          </div>

          {isLoading && (
            <div className="flex items-center justify-center rounded-lg border border-zinc-200 bg-white p-12 dark:border-zinc-800 dark:bg-black">
              <div className="text-center">
                <div className="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-zinc-200 border-t-zinc-900 dark:border-zinc-800 dark:border-t-zinc-50"></div>
                <p className="mt-4 text-sm text-zinc-600 dark:text-zinc-400">
                  Loading transaction history...
                </p>
              </div>
            </div>
          )}

          {error && (
            <div className="rounded-lg border border-yellow-200 bg-yellow-50 p-4 dark:border-yellow-900 dark:bg-yellow-950">
              <div className="flex items-start gap-3">
                <svg className="h-5 w-5 flex-shrink-0 text-yellow-600 dark:text-yellow-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <div>
                  <p className="text-sm font-medium text-yellow-800 dark:text-yellow-200">
                    Transaction history temporarily unavailable
                  </p>
                  <p className="mt-1 text-xs text-yellow-700 dark:text-yellow-300">
                    {error}. The vault is still functional - only historical data display is affected.
                  </p>
                </div>
              </div>
            </div>
          )}

          {!isLoading && events.length === 0 && !error && (
            <div className="rounded-lg border border-zinc-200 bg-white p-12 text-center dark:border-zinc-800 dark:bg-black">
              <svg className="mx-auto h-12 w-12 text-zinc-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
              </svg>
              <h3 className="mt-4 text-lg font-medium text-zinc-900 dark:text-zinc-50">
                No activity yet
              </h3>
              <p className="mt-2 text-sm text-zinc-600 dark:text-zinc-400">
                Vault activity will appear here once deposits or trades are made.
              </p>
              <p className="mt-2 text-xs text-zinc-500 dark:text-zinc-500">
                Make the first deposit to get started!
              </p>
            </div>
          )}

          {!isLoading && !error && events.length > 0 && (
            <div className="space-y-3">
              {events.map((event) => (
                <div
                  key={event.id}
                  className="rounded-lg border border-zinc-200 bg-white p-4 transition-colors hover:border-zinc-300 dark:border-zinc-800 dark:bg-black dark:hover:border-zinc-700"
                >
                  <div className="flex items-start gap-4">
                    <div className="mt-1 flex-shrink-0">
                      {getEventIcon(event.type)}
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex-1">
                          <h3 className="text-sm font-medium text-zinc-900 dark:text-zinc-50">
                            {getEventTitle(event)}
                          </h3>
                          <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
                            {getEventDescription(event)}
                          </p>
                        </div>

                        <div className="flex-shrink-0 text-right">
                          <p className="text-xs text-zinc-500 dark:text-zinc-500">
                            {formatTimestamp(event.timestamp)}
                          </p>
                          <p className="mt-1 text-xs text-zinc-400 dark:text-zinc-600">
                            Block {event.blockHeight.toLocaleString()}
                          </p>
                        </div>
                      </div>

                      {/* Transaction hash link */}
                      <div className="mt-2">
                        <a
                          href={`https://neutron.celat.one/neutron-1/txs/${event.txHash}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="inline-flex items-center gap-1 text-xs text-zinc-500 transition-colors hover:text-zinc-900 dark:text-zinc-500 dark:hover:text-zinc-50"
                        >
                          <span className="font-mono">{event.txHash.slice(0, 8)}...{event.txHash.slice(-6)}</span>
                          <svg className="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                          </svg>
                        </a>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
