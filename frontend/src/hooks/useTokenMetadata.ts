"use client";

import { useState, useEffect } from "react";
import { useWallet } from "../contexts/WalletContext";
import { getDenomMetadata } from "interchainjs/cosmos/bank/v1beta1/query.rpc.func";

interface TokenMetadata {
  denom: string;
  symbol: string;
  name: string;
  decimals: number;
  display: string;
}

const metadataCache = new Map<string, TokenMetadata>();

export function useTokenMetadata(denoms: string[]) {
  const [metadata, setMetadata] = useState<Map<string, TokenMetadata>>(
    new Map()
  );
  const [isLoading, setIsLoading] = useState(false);
  const { rpcEndpoint } = useWallet();

  useEffect(() => {
    const fetchMetadata = async () => {
      if (denoms.length === 0) return;

      setIsLoading(true);
      const newMetadata = new Map<string, TokenMetadata>();

      for (const denom of denoms) {
        // Check cache first
        if (metadataCache.has(denom)) {
          newMetadata.set(denom, metadataCache.get(denom)!);
          continue;
        }

        try {
          // Fetch from bank module via RPC
          const response = await getDenomMetadata(rpcEndpoint, { denom });

          if (response.metadata) {
            const meta = response.metadata;
            console.log(meta);

            const tokenMeta: TokenMetadata = {
              denom,
              symbol: meta.symbol || getDenomSymbol(denom),
              name: meta.name || meta.display || getDenomSymbol(denom),
              decimals:
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                meta.denomUnits?.find((u: any) => u.denom === meta.display)
                  ?.exponent || 6,
              display: meta.display || getDenomSymbol(denom),
            };

            metadataCache.set(denom, tokenMeta);
            newMetadata.set(denom, tokenMeta);
          } else {
            // Fallback to parsing the denom
            const tokenMeta = createFallbackMetadata(denom);
            metadataCache.set(denom, tokenMeta);
            newMetadata.set(denom, tokenMeta);
          }
        } catch (error) {
          console.error(`Failed to fetch metadata for ${denom}:`, error);
          // Fallback to parsing the denom
          const tokenMeta = createFallbackMetadata(denom);
          metadataCache.set(denom, tokenMeta);
          newMetadata.set(denom, tokenMeta);
        }
      }

      setMetadata(newMetadata);
      setIsLoading(false);
    };

    fetchMetadata();
  }, [denoms.join(","), rpcEndpoint]);

  return { metadata, isLoading };
}

function getDenomSymbol(denom: string): string {
  // Remove common prefixes
  if (denom.startsWith("ibc/")) {
    return `IBC/${denom.slice(4, 12).toUpperCase()}`;
  }

  if (denom.startsWith("factory/")) {
    // factory/neutron.../token -> TOKEN
    const parts = denom.split("/");
    return parts[parts.length - 1].toUpperCase();
  }

  // Handle micro denoms (uatom -> ATOM)
  if (denom.startsWith("u")) {
    return denom.slice(1).toUpperCase();
  }

  return denom.toUpperCase();
}

function createFallbackMetadata(denom: string): TokenMetadata {
  return {
    denom,
    symbol: getDenomSymbol(denom),
    name: getDenomSymbol(denom),
    decimals: 6,
    display: getDenomSymbol(denom),
  };
}

export function formatTokenAmount(
  amount: string,
  decimals: number = 6
): string {
  try {
    const num = parseFloat(amount) / Math.pow(10, decimals);
    if (num >= 1000000) {
      return `${(num / 1000000).toFixed(2)}M`;
    } else if (num >= 1000) {
      return `${(num / 1000).toFixed(2)}K`;
    }
    if (10 >= num && num > 0) {
      return num.toFixed(decimals);
    }
    return num.toFixed(2);
  } catch {
    return "0.00";
  }
}
