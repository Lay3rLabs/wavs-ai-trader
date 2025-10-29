"use client";

import { useState, useEffect } from "react";
import { useWallet } from "../contexts/WalletContext";
import {
  VaultQueryClient,
  VaultClient,
} from "../contract-bindings/Vault.client";
import { getCosmWasmClient } from "../contract-bindings/baseClient";
import type {
  VaultState,
  DepositRequest,
  Coin,
} from "../contract-bindings/Vault.types";

const VAULT_CONTRACT_ADDRESS =
  process.env.NEXT_PUBLIC_VAULT_CONTRACT_ADDRESS || "";

export function useVault() {
  const { address, client, rpcEndpoint, isConnected } = useWallet();
  const [vaultState, setVaultState] = useState<VaultState | null>(null);
  const [whitelistedDenoms, setWhitelistedDenoms] = useState<string[]>([]);
  const [pendingDeposits, setPendingDeposits] = useState<DepositRequest[]>([]);
  const [userShares, setUserShares] = useState<string>("0");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Create query client
  const queryClient = new VaultQueryClient(
    getCosmWasmClient(rpcEndpoint),
    VAULT_CONTRACT_ADDRESS
  );

  // Fetch vault data
  const fetchVaultData = async () => {
    if (!VAULT_CONTRACT_ADDRESS) return;

    setIsLoading(true);
    setError(null);
    try {
      const [state, denoms, deposits] = await Promise.all([
        queryClient.getVaultState(),
        queryClient.getWhitelistedDenoms(),
        queryClient.listDepositRequests({ limit: 10 }),
      ]);

      console.log('All deposits:', deposits);
      console.log('Deposits with states:', deposits.map(d => ({ id: d.id, state: d.state })));

      setVaultState(state);
      setWhitelistedDenoms(denoms);

      // Filter for pending deposits - state can be either "pending" string or object without completed
      const pending = deposits.filter((d) => {
        // Handle both string "pending" and object form
        if (typeof d.state === 'string') {
          return d.state === "pending";
        }
        // If it's an object, check if it doesn't have "completed" property
        return !('completed' in d.state);
      });

      console.log('Filtered pending deposits:', pending);
      setPendingDeposits(pending);
    } catch (err) {
      console.error("Error fetching vault data:", err);
      setError(
        err instanceof Error ? err.message : "Failed to fetch vault data"
      );
    } finally {
      setIsLoading(false);
    }
  };

  // Fetch user shares
  const fetchUserShares = async () => {
    if (!address || !VAULT_CONTRACT_ADDRESS) return;

    try {
      // Note: This is a simplified version. In a real implementation,
      // you'd need to track user shares in the contract or query from events
      // For now, we'll just set it to total shares if user has any deposits
      const totalShares = await queryClient.getTotalShares();
      const deposits = await queryClient.listDepositRequests({ limit: 100 });
      const userDeposits = deposits.filter((d) => d.user === address);

      if (userDeposits.length > 0) {
        // This is a simplification - in production you'd track this properly
        setUserShares(totalShares);
      }
    } catch (err) {
      console.error("Error fetching user shares:", err);
    }
  };

  // Deposit tokens
  const deposit = async (amount: string, denom: string) => {
    console.log("Deposit called with:", { amount, denom, client, address });
    if (!client || !address) {
      console.error("Wallet not connected. Client:", client, "Address:", address);
      throw new Error("Wallet not connected");
    }

    try {
      const vaultClient = new VaultClient(
        client,
        address,
        VAULT_CONTRACT_ADDRESS
      );
      const funds: Coin[] = [{ amount, denom }];

      console.log("Calling vaultClient.deposit with funds:", funds);
      const result = await vaultClient.deposit("auto", undefined, funds);
      console.log("Deposit transaction result (full):", JSON.stringify(result, null, 2));
      console.log("Result code:", result?.code);
      console.log("Result rawLog:", result?.rawLog);

      // Check for transaction failure - only fail if we have explicit error indicators
      if (result?.code !== undefined && result?.code !== 0) {
        const errorMsg = result?.rawLog || result?.log || 'Transaction failed';
        console.error("Transaction failed with code:", result.code, errorMsg);
        throw new Error(`Transaction failed: ${errorMsg}`);
      }

      // Additional check for empty result
      if (!result || (typeof result === 'object' && Object.keys(result).length === 0)) {
        console.error("Empty transaction result received");
        throw new Error("Transaction failed: No result returned");
      }

      console.log("Transaction succeeded!");
      await fetchVaultData();
      return result;
    } catch (error) {
      console.error("Deposit transaction error:", error);
      throw error;
    }
  };

  // Withdraw shares
  const withdraw = async (shares: string) => {
    console.log("Withdraw called with:", { shares, client, address });
    if (!client || !address) {
      console.error("Wallet not connected. Client:", client, "Address:", address);
      throw new Error("Wallet not connected");
    }

    try {
      console.log("Client object:", client);

      const vaultClient = new VaultClient(
        client,
        address,
        VAULT_CONTRACT_ADDRESS
      );

      console.log("Calling vaultClient.withdraw with shares:", shares);
      const result = await vaultClient.withdraw({ shares }, "auto");
      console.log("Withdraw transaction result (full):", JSON.stringify(result, null, 2));
      console.log("Result code:", result?.code);
      console.log("Result rawLog:", result?.rawLog);

      // Check for transaction failure - only fail if we have explicit error indicators
      if (result?.code !== undefined && result?.code !== 0) {
        const errorMsg = result?.rawLog || result?.log || 'Transaction failed';
        console.error("Transaction failed with code:", result.code, errorMsg);
        throw new Error(`Transaction failed: ${errorMsg}`);
      }

      // Additional check for empty result
      if (!result || (typeof result === 'object' && Object.keys(result).length === 0)) {
        console.error("Empty transaction result received");
        throw new Error("Transaction failed: No result returned");
      }

      console.log("Transaction succeeded!");
      await fetchVaultData();
      await fetchUserShares();
      return result;
    } catch (error) {
      console.error("Withdraw transaction error:", error);
      throw error;
    }
  };

  // Refresh data periodically
  useEffect(() => {
    if (VAULT_CONTRACT_ADDRESS) {
      fetchVaultData();
      const interval = setInterval(fetchVaultData, 30000); // Refresh every 30s
      return () => clearInterval(interval);
    }
  }, []);

  useEffect(() => {
    if (isConnected) {
      fetchUserShares();
    }
  }, [isConnected, address]);

  return {
    vaultState,
    whitelistedDenoms,
    pendingDeposits,
    userShares,
    isLoading,
    error,
    deposit,
    withdraw,
    refresh: fetchVaultData,
  };
}
