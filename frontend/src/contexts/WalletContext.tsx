/* eslint-disable @typescript-eslint/no-explicit-any */
"use client";

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  ReactNode,
} from "react";
import { DirectSigner, createCosmosQueryClient } from "@interchainjs/cosmos";
import {
  getSigningCosmWasmClient,
  ISigningCosmWasmClient,
} from "../contract-bindings/baseClient";

interface Window {
  keplr?: any;
}

interface WalletContextType {
  address: string | null;
  isConnected: boolean;
  isConnecting: boolean;
  connect: () => Promise<void>;
  disconnect: () => void;
  client: ISigningCosmWasmClient | null;
  chainId: string;
  rpcEndpoint: string;
}

const WalletContext = createContext<WalletContextType | undefined>(undefined);

// Configuration - Update these for your chain
const CHAIN_ID = process.env.NEXT_PUBLIC_CHAIN_ID || "neutron-1";
const RPC_ENDPOINT =
  process.env.NEXT_PUBLIC_RPC_ENDPOINT ||
  "https://neutron-rpc.publicnode.com:443";

export function WalletProvider({ children }: { children: ReactNode }) {
  const [address, setAddress] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);
  const [client, setClient] = useState<ISigningCosmWasmClient | null>(null);

  // Check if wallet is already connected on mount
  useEffect(() => {
    const checkConnection = async () => {
      if (typeof window !== "undefined" && (window as any).keplr) {
        try {
          const key = await (window as any).keplr.getKey(CHAIN_ID);
          if (key) {
            setAddress(key.bech32Address);
            await initializeClient(key.bech32Address);
          }
        } catch (error) {
          // Wallet not connected or chain not added
          console.log("Wallet not connected:", error);
        }
      }
    };
    checkConnection();
  }, []);

  const initializeClient = async (userAddress: string) => {
    if (typeof window !== "undefined" && (window as any).keplr) {
      try {
        console.log("Initializing client for address:", userAddress);
        const offlineSigner = await (window as any).keplr.getOfflineSigner(
          CHAIN_ID
        );
        console.log("Got offline signer");

        // Create a query client for the DirectSigner
        console.log("Creating query client with RPC:", RPC_ENDPOINT);
        let queryClient;
        try {
          queryClient = await createCosmosQueryClient(RPC_ENDPOINT, {
            timeout: 10000
          });
          console.log("Query client created successfully");
        } catch (queryError) {
          console.error("Failed to create query client:", queryError);
          // Try with a different approach - just use the RPC endpoint directly
          throw new Error(`Cannot connect to RPC endpoint ${RPC_ENDPOINT}. Please check your network connection.`);
        }

        // Create a proper DirectSigner with the offline signer and query client
        const directSigner = new DirectSigner(offlineSigner, {
          queryClient
        });
        console.log("DirectSigner created");

        const signingClient = getSigningCosmWasmClient(
          directSigner,
          RPC_ENDPOINT
        );
        console.log("Signing client created");
        setClient(signingClient);
        console.log("Client initialized successfully");
      } catch (error) {
        console.error("Failed to initialize client:", error);
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        console.error("Error details:", errorMessage);
        // Don't show alert on initial load, only on explicit connection attempt
      }
    }
  };

  const connect = async () => {
    if (typeof window === "undefined") return;

    setIsConnecting(true);
    try {
      const keplr = (window as any).keplr;

      if (!keplr) {
        alert("Please install Keplr extension");
        window.open("https://www.keplr.app/", "_blank");
        setIsConnecting(false);
        return;
      }

      // Try to enable the chain
      try {
        await keplr.enable(CHAIN_ID);
      } catch (error) {
        // If chain is not added, suggest adding it
        console.error("Failed to enable chain:", error);
        alert(`Please add ${CHAIN_ID} to Keplr`);
        setIsConnecting(false);
        return;
      }

      const key = await keplr.getKey(CHAIN_ID);
      setAddress(key.bech32Address);

      await initializeClient(key.bech32Address);
    } catch (error) {
      console.error("Failed to connect wallet:", error);
      alert("Failed to connect wallet. Please try again.");
    } finally {
      setIsConnecting(false);
    }
  };

  const disconnect = () => {
    setAddress(null);
    setClient(null);
  };

  return (
    <WalletContext.Provider
      value={{
        address,
        isConnected: !!address,
        isConnecting,
        connect,
        disconnect,
        client,
        chainId: CHAIN_ID,
        rpcEndpoint: RPC_ENDPOINT,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (context === undefined) {
    throw new Error("useWallet must be used within a WalletProvider");
  }
  return context;
}
