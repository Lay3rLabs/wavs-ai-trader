"use client";

import { useState, useEffect } from "react";
import { createCosmosQueryClient } from "@interchainjs/cosmos";

const VAULT_CONTRACT_ADDRESS =
  process.env.NEXT_PUBLIC_VAULT_CONTRACT_ADDRESS || "";

interface TradeData {
  spentRaw?: string;
  spentAmount?: string;
  spentDenom?: string;
  spender?: string;
  receivedRaw?: string;
  receivedAmount?: string;
  receivedDenom?: string;
  receiver?: string;
}

interface BlockchainEvent {
  type: string;
  attributes?: Array<{ key: string; value: string }>;
}

interface ParsedEvent {
  id: string;
  type:
    | "deposit"
    | "deposit_processed"
    | "withdraw"
    | "price_update"
    | "rebalancing"
    | "unknown";
  timestamp: string;
  blockHeight: number;
  txHash: string;
  data: Record<string, string>;
}

export function useTransactionHistory() {
  const [events, setEvents] = useState<ParsedEvent[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchTransactionHistory();
  }, []);

  const fetchTransactionHistory = async () => {
    if (!VAULT_CONTRACT_ADDRESS) {
      console.log("No vault contract address configured");
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const rpcEndpoint =
        process.env.NEXT_PUBLIC_RPC_ENDPOINT ||
        "https://neutron-rpc.publicnode.com:443";
      const queryClient = await createCosmosQueryClient(rpcEndpoint);

      // Query all transactions for this contract
      const result = await queryClient.searchTxs({
        query: `wasm._contract_address='${VAULT_CONTRACT_ADDRESS}'`,
        page: 1,
        perPage: 50,
        orderBy: "desc", // newest first
      });

      console.log("Transaction search result:", result);

      // Parse events from transactions
      const parsedEvents: ParsedEvent[] = [];

      // Handle case where there are no transactions yet
      if (!result.txs) {
        console.log(
          "No transactions found for contract (txs is null/undefined)",
        );
        setEvents([]);
        return;
      }

      if (!Array.isArray(result.txs)) {
        console.error(
          "Invalid transaction result structure - txs is not an array:",
          result,
        );
        throw new Error("Invalid transaction data received");
      }

      if (result.txs.length === 0) {
        console.log("No transactions found for contract (empty array)");
        setEvents([]);
        return;
      }

      for (const tx of result.txs) {
        console.log("Processing tx with keys:", Object.keys(tx));

        // Convert hash from Uint8Array to hex string
        let txHash = "";
        if (tx.hash) {
          if (tx.hash instanceof Uint8Array) {
            txHash = Array.from<number>(tx.hash as Uint8Array)
              .map((b) => b.toString(16).padStart(2, "0"))
              .join("")
              .toUpperCase();
          } else if (typeof tx.hash === "string") {
            txHash = tx.hash;
          }
        }
        console.log("TX Hash:", txHash);

        // Extract timestamp from transaction
        const timestamp = tx.timestamp || new Date().toISOString();
        const blockHeight = tx.height || 0;

        // Handle different possible event structures - check ALL possible locations
        let events = [];
        if (
          tx.txResult &&
          tx.txResult.events &&
          Array.isArray(tx.txResult.events)
        ) {
          console.log("Found events at tx.txResult.events");
          events = tx.txResult.events;
        } else if (tx.events && Array.isArray(tx.events)) {
          console.log("Found events at tx.events");
          events = tx.events;
        } else if (
          tx.result &&
          tx.result.events &&
          Array.isArray(tx.result.events)
        ) {
          console.log("Found events at tx.result.events");
          events = tx.result.events;
        } else if (
          tx.tx_result &&
          tx.tx_result.events &&
          Array.isArray(tx.tx_result.events)
        ) {
          console.log("Found events at tx.tx_result.events");
          events = tx.tx_result.events;
        } else if (tx.tx && tx.tx.events && Array.isArray(tx.tx.events)) {
          console.log("Found events at tx.tx.events");
          events = tx.tx.events;
        } else if (
          tx.txResponse &&
          tx.txResponse.events &&
          Array.isArray(tx.txResponse.events)
        ) {
          console.log("Found events at tx.txResponse.events");
          events = tx.txResponse.events;
        } else if (
          tx.tx_response &&
          tx.tx_response.events &&
          Array.isArray(tx.tx_response.events)
        ) {
          console.log("Found events at tx.tx_response.events");
          events = tx.tx_response.events;
        } else {
          console.warn("No events found in tx.");
          // Try to log all nested objects to find where events might be (no stringify to avoid BigInt error)
          console.log("Checking nested structures:");
          if (tx.result) console.log("tx.result keys:", Object.keys(tx.result));
          if (tx.tx_result)
            console.log("tx.tx_result keys:", Object.keys(tx.tx_result));
          if (tx.tx) console.log("tx.tx keys:", Object.keys(tx.tx));
          if (tx.txResult)
            console.log("tx.txResult keys:", Object.keys(tx.txResult));
          if (tx.txResponse)
            console.log("tx.txResponse keys:", Object.keys(tx.txResponse));
          if (tx.tx_response)
            console.log("tx.tx_response keys:", Object.keys(tx.tx_response));
          continue;
        }

        console.log(`Found ${events.length} events in transaction`);

        console.log(
          "Event types:",
          events.map((e: BlockchainEvent) => e?.type),
        );

        // Check if this transaction has both coin_spent and coin_received events (indicates a trade/swap)
        const hasCoinSpent = events.some(
          (e: BlockchainEvent) => e.type === "coin_spent",
        );

        const hasCoinReceived = events.some(
          (e: BlockchainEvent) => e.type === "coin_received",
        );

        if (hasCoinSpent && hasCoinReceived) {
          console.log(
            "Found trade transaction with coin_spent and coin_received",
          );

          const tradeData: TradeData = {};

          for (const event of events) {
            if (!event || typeof event !== "object") {
              continue;
            }

            const eventType = event.type || "";

            // Process coin_spent and coin_received events
            if (eventType === "coin_spent" || eventType === "coin_received") {
              // Extract attributes into a map
              const attrs: Record<string, string> = {};

              // Handle different attribute structures
              const attributes = event.attributes || [];
              if (Array.isArray(attributes)) {
                for (const attr of attributes) {
                  if (attr && typeof attr === "object") {
                    let key = attr.key || "";
                    let value = attr.value || "";

                    // Decode base64 key first
                    try {
                      const decodedKey = atob(key);
                      // Check if decoded key is printable ASCII (indicates text, not binary)
                      if (decodedKey && /^[\x20-\x7E]+$/.test(decodedKey)) {
                        key = decodedKey;
                      }
                    } catch {
                      // Not base64, use as is
                    }

                    // Only decode value if it's for known text attributes
                    const textAttributes = ["spender", "receiver", "amount"];
                    if (textAttributes.includes(key)) {
                      try {
                        const decodedValue = atob(value);
                        // Check if it's printable ASCII or contains expected chars for cosmos addresses/amounts
                        if (
                          decodedValue &&
                          /^[\x20-\x7E]+$/.test(decodedValue)
                        ) {
                          value = decodedValue;
                        }
                      } catch {
                        // Not base64, use as is
                      }
                    }

                    attrs[key] = value;
                  }
                }
              }

              console.log(`${eventType} attributes:`, attrs);

              // Check if this event is related to our vault contract
              const spender = attrs.spender || "";
              const receiver = attrs.receiver || "";

              if (
                spender === VAULT_CONTRACT_ADDRESS ||
                receiver === VAULT_CONTRACT_ADDRESS
              ) {
                // Parse the amount string (format: "1000uatom" or "1000000,2000000uosmo")
                const amount = attrs.amount || "";
                console.log(`Processing ${eventType} amount:`, amount);

                if (eventType === "coin_spent") {
                  tradeData.spentRaw = amount;
                  tradeData.spender = spender;
                  // Parse amount and denom
                  const match = amount.match(/^(\d+)([a-z][a-z0-9/-]+)$/i);
                  if (match) {
                    tradeData.spentAmount = match[1];
                    tradeData.spentDenom = match[2];
                  }
                } else if (eventType === "coin_received") {
                  tradeData.receivedRaw = amount;
                  tradeData.receiver = receiver;
                  // Parse amount and denom
                  const match = amount.match(/^(\d+)([a-z][a-z0-9/-]+)$/i);
                  if (match) {
                    tradeData.receivedAmount = match[1];
                    tradeData.receivedDenom = match[2];
                  }
                }
              }
            }
          }

          // If we found both spent and received for our vault, create a trade event
          if (tradeData.spentAmount && tradeData.receivedAmount) {
            console.log("Creating trade event:", tradeData);
            const parsed: ParsedEvent = {
              id: `${txHash}-trade-${blockHeight}`,
              type: "rebalancing",
              timestamp,
              blockHeight,
              txHash,
              data: {
                from: `${tradeData.spentAmount} ${tradeData.spentDenom}`,
                to: `${tradeData.receivedAmount} ${tradeData.receivedDenom}`,
                fromDenom: tradeData.spentDenom || "",
                toDenom: tradeData.receivedDenom || "",
                fromAmount: tradeData.spentAmount,
                toAmount: tradeData.receivedAmount,
              },
            };
            parsedEvents.push(parsed);
          }
        }
      }

      console.log("Parsed events:", parsedEvents);
      setEvents(parsedEvents);
    } catch (err) {
      console.error("Error fetching transaction history:", err);
      console.error("Error details:", JSON.stringify(err, null, 2));
      setError(
        err instanceof Error
          ? err.message
          : "Failed to fetch transaction history",
      );
      // Set empty array on error so component can still render
      setEvents([]);
    } finally {
      setIsLoading(false);
    }
  };

  return {
    events,
    isLoading,
    error,
    refresh: fetchTransactionHistory,
  };
}
