"use client";

import { useState } from 'react';
import { useWallet } from '../contexts/WalletContext';
import { useVault } from '../hooks/useVault';
import { useTokenMetadata } from '../hooks/useTokenMetadata';

export function DepositWithdraw() {
  const { isConnected } = useWallet();
  const { whitelistedDenoms, userShares, deposit, withdraw, isLoading } = useVault();
  const { metadata: tokenMetadata } = useTokenMetadata(whitelistedDenoms);

  const [activeTab, setActiveTab] = useState<'deposit' | 'withdraw'>('deposit');
  const [selectedDenom, setSelectedDenom] = useState('');
  const [amount, setAmount] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const handleDeposit = async () => {
    if (!selectedDenom || !amount) {
      setError('Please select a token and enter an amount');
      return;
    }

    setIsSubmitting(true);
    setError(null);
    setSuccess(null);

    try {
      // Convert amount to microunits (multiply by 1,000,000)
      const microAmount = (parseFloat(amount) * 1_000_000).toString();
      await deposit(microAmount, selectedDenom);
      setSuccess('Deposit submitted successfully! Waiting for price update to process.');
      setAmount('');
    } catch (err) {
      console.error('Deposit error:', err);
      setError(err instanceof Error ? err.message : 'Failed to deposit');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleWithdraw = async () => {
    if (!amount) {
      setError('Please enter an amount of shares to withdraw');
      return;
    }

    setIsSubmitting(true);
    setError(null);
    setSuccess(null);

    try {
      await withdraw(amount);
      setSuccess('Withdrawal successful!');
      setAmount('');
    } catch (err) {
      console.error('Withdrawal error:', err);
      setError(err instanceof Error ? err.message : 'Failed to withdraw');
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isConnected) {
    return (
      <div className="border-b border-zinc-200 bg-white dark:border-zinc-800 dark:bg-black">
        <div className="container mx-auto px-4 py-12 sm:px-6 lg:px-8">
          <div className="mx-auto max-w-2xl rounded-lg border border-zinc-200 bg-zinc-50 p-8 text-center dark:border-zinc-800 dark:bg-zinc-900">
            <svg className="mx-auto h-12 w-12 text-zinc-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
            <h3 className="mt-4 text-lg font-semibold text-zinc-900 dark:text-zinc-50">
              Connect Your Wallet
            </h3>
            <p className="mt-2 text-sm text-zinc-600 dark:text-zinc-400">
              Please connect your wallet to deposit or withdraw from the vault.
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="border-b border-zinc-200 bg-white dark:border-zinc-800 dark:bg-black">
      <div className="container mx-auto px-4 py-12 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl">
          <h2 className="mb-6 text-2xl font-semibold text-zinc-900 dark:text-zinc-50">
            Manage Your Position
          </h2>

          {/* Tabs */}
          <div className="mb-6 flex gap-2 rounded-lg border border-zinc-200 bg-zinc-50 p-1 dark:border-zinc-800 dark:bg-zinc-900">
            <button
              onClick={() => {
                setActiveTab('deposit');
                setError(null);
                setSuccess(null);
              }}
              className={`flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
                activeTab === 'deposit'
                  ? 'bg-white text-zinc-900 shadow-sm dark:bg-zinc-950 dark:text-zinc-50'
                  : 'text-zinc-600 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-50'
              }`}
            >
              Deposit
            </button>
            <button
              onClick={() => {
                setActiveTab('withdraw');
                setError(null);
                setSuccess(null);
              }}
              className={`flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
                activeTab === 'withdraw'
                  ? 'bg-white text-zinc-900 shadow-sm dark:bg-zinc-950 dark:text-zinc-50'
                  : 'text-zinc-600 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-50'
              }`}
            >
              Withdraw
            </button>
          </div>

          {/* Deposit Form */}
          {activeTab === 'deposit' && (
            <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-6 dark:border-zinc-800 dark:bg-zinc-900">
              <div className="space-y-4">
                <div>
                  <label className="mb-2 block text-sm font-medium text-zinc-700 dark:text-zinc-300">
                    Select Token
                  </label>
                  <select
                    value={selectedDenom}
                    onChange={(e) => setSelectedDenom(e.target.value)}
                    className="w-full rounded-lg border border-zinc-300 bg-white px-4 py-2.5 text-zinc-900 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-50"
                    disabled={isLoading || isSubmitting}
                  >
                    <option value="">Choose a token...</option>
                    {whitelistedDenoms.map((denom) => {
                      const meta = tokenMetadata.get(denom);
                      return (
                        <option key={denom} value={denom}>
                          {meta?.symbol || denom.replace('u', '').toUpperCase()}
                          {meta?.name && meta.name !== meta.symbol && ` - ${meta.name}`}
                        </option>
                      );
                    })}
                  </select>
                </div>

                <div>
                  <label className="mb-2 block text-sm font-medium text-zinc-700 dark:text-zinc-300">
                    Amount
                  </label>
                  <input
                    type="number"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    placeholder="0.00"
                    min="0"
                    step="0.000001"
                    className="w-full rounded-lg border border-zinc-300 bg-white px-4 py-2.5 text-zinc-900 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-50"
                    disabled={isLoading || isSubmitting}
                  />
                  <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">
                    Enter amount in token units (e.g., 100 ATOM)
                  </p>
                </div>

                {error && (
                  <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200">
                    {error}
                  </div>
                )}

                {success && (
                  <div className="rounded-lg border border-green-200 bg-green-50 p-3 text-sm text-green-800 dark:border-green-900 dark:bg-green-950 dark:text-green-200">
                    {success}
                  </div>
                )}

                <button
                  onClick={handleDeposit}
                  disabled={isLoading || isSubmitting || !selectedDenom || !amount}
                  className="w-full rounded-lg bg-zinc-900 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-zinc-800 disabled:opacity-50 disabled:cursor-not-allowed dark:bg-zinc-50 dark:text-zinc-900 dark:hover:bg-zinc-200"
                >
                  {isSubmitting ? 'Depositing...' : 'Deposit Tokens'}
                </button>

                <div className="rounded-lg border border-blue-200 bg-blue-50 p-3 dark:border-blue-900 dark:bg-blue-950">
                  <p className="text-xs text-blue-800 dark:text-blue-200">
                    Your deposit will be pending until the next price update from the WAVS AI agents.
                    Once processed, you will receive vault shares proportional to your deposit value.
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Withdraw Form */}
          {activeTab === 'withdraw' && (
            <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-6 dark:border-zinc-800 dark:bg-zinc-900">
              <div className="space-y-4">
                <div>
                  <div className="mb-2 flex items-center justify-between">
                    <label className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
                      Shares to Withdraw
                    </label>
                    <span className="text-xs text-zinc-600 dark:text-zinc-400">
                      Available: {userShares}
                    </span>
                  </div>
                  <input
                    type="text"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    placeholder="0"
                    className="w-full rounded-lg border border-zinc-300 bg-white px-4 py-2.5 text-zinc-900 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-50"
                    disabled={isLoading || isSubmitting}
                  />
                  <p className="mt-1 text-xs text-zinc-500 dark:text-zinc-500">
                    You will receive a proportional share of all vault assets
                  </p>
                </div>

                {error && (
                  <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800 dark:border-red-900 dark:bg-red-950 dark:text-red-200">
                    {error}
                  </div>
                )}

                {success && (
                  <div className="rounded-lg border border-green-200 bg-green-50 p-3 text-sm text-green-800 dark:border-green-900 dark:bg-green-950 dark:text-green-200">
                    {success}
                  </div>
                )}

                <button
                  onClick={handleWithdraw}
                  disabled={isLoading || isSubmitting || !amount}
                  className="w-full rounded-lg bg-zinc-900 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-zinc-800 disabled:opacity-50 disabled:cursor-not-allowed dark:bg-zinc-50 dark:text-zinc-900 dark:hover:bg-zinc-200"
                >
                  {isSubmitting ? 'Withdrawing...' : 'Withdraw Shares'}
                </button>

                <div className="rounded-lg border border-blue-200 bg-blue-50 p-3 dark:border-blue-900 dark:bg-blue-950">
                  <p className="text-xs text-blue-800 dark:text-blue-200">
                    Withdrawing shares instantly returns your proportional share of all assets in the vault.
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
