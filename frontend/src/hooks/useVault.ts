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

      setVaultState(state);
      setWhitelistedDenoms(denoms);
      setPendingDeposits(deposits.filter((d) => d.state === "pending"));
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

    const vaultClient = new VaultClient(
      client,
      address,
      VAULT_CONTRACT_ADDRESS
    );
    const funds: Coin[] = [{ amount, denom }];

    const result = await vaultClient.deposit("auto", undefined, funds);
    await fetchVaultData();
    return result;
  };

  // Withdraw shares
  const withdraw = async (shares: string) => {
    console.log("Withdraw called with:", { shares, client, address });
    if (!client || !address) {
      console.error("Wallet not connected. Client:", client, "Address:", address);
      throw new Error("Wallet not connected");
    }

    console.log("Client object:", client);

    const vaultClient = new VaultClient(
      client,
      address,
      VAULT_CONTRACT_ADDRESS
    );
    const result = await vaultClient.withdraw({ shares }, "auto");
    await fetchVaultData();
    await fetchUserShares();
    return result;
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
