"use client";

import { useState, useEffect } from 'react';
import { createCosmosQueryClient } from '@interchainjs/cosmos';

const VAULT_CONTRACT_ADDRESS = process.env.NEXT_PUBLIC_VAULT_CONTRACT_ADDRESS || '';

interface TransactionEvent {
  type: string;
  attributes: Array<{ key: string; value: string }>;
}

interface Transaction {
  hash: string;
  height: number;
  timestamp: string;
  events: TransactionEvent[];
  code: number;
}

interface ParsedEvent {
  id: string;
  type: 'deposit' | 'deposit_processed' | 'withdraw' | 'price_update' | 'rebalancing' | 'unknown';
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
      console.log('No vault contract address configured');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const rpcEndpoint = process.env.NEXT_PUBLIC_RPC_ENDPOINT || 'https://neutron-rpc.publicnode.com:443';
      const queryClient = await createCosmosQueryClient(rpcEndpoint);

      // Query all transactions for this contract
      const result = await queryClient.searchTxs({
        query: `wasm._contract_address='${VAULT_CONTRACT_ADDRESS}'`,
        page: 1,
        perPage: 50,
        orderBy: 'desc', // newest first
      });

      console.log('Transaction search result:', result);

      // Parse events from transactions
      const parsedEvents: ParsedEvent[] = [];

      // Handle case where there are no transactions yet
      if (!result.txs) {
        console.log('No transactions found for contract (txs is null/undefined)');
        setEvents([]);
        return;
      }

      if (!Array.isArray(result.txs)) {
        console.error('Invalid transaction result structure - txs is not an array:', result);
        throw new Error('Invalid transaction data received');
      }

      if (result.txs.length === 0) {
        console.log('No transactions found for contract (empty array)');
        setEvents([]);
        return;
      }

      for (const tx of result.txs) {
        console.log('Processing tx:', tx);

        // Extract timestamp from transaction
        const timestamp = tx.timestamp || new Date().toISOString();

        // Handle different possible event structures
        let events = [];
        if (tx.events && Array.isArray(tx.events)) {
          events = tx.events;
        } else if (tx.result && tx.result.events && Array.isArray(tx.result.events)) {
          events = tx.result.events;
        } else if (tx.tx_result && tx.tx_result.events && Array.isArray(tx.tx_result.events)) {
          events = tx.tx_result.events;
        } else {
          console.warn('No events found in tx:', tx);
          continue;
        }

        // Look through all events in the transaction
        for (const event of events) {
          if (!event || typeof event !== 'object') {
            continue;
          }

          const eventType = event.type || '';

          if (eventType.startsWith('wasm') || eventType === 'execute' || eventType === 'wasm') {
            // Extract attributes into a map
            const attrs: Record<string, string> = {};

            // Handle different attribute structures
            const attributes = event.attributes || [];
            if (Array.isArray(attributes)) {
              for (const attr of attributes) {
                if (attr && typeof attr === 'object') {
                  const key = attr.key || '';
                  const value = attr.value || '';
                  attrs[key] = value;
                }
              }
            }

            // Only process events from our contract
            if (attrs._contract_address === VAULT_CONTRACT_ADDRESS || attrs.contract_address === VAULT_CONTRACT_ADDRESS) {
              const parsed = parseEvent(event, tx.hash, tx.height, timestamp, attrs);
              if (parsed) {
                parsedEvents.push(parsed);
              }
            }
          }
        }
      }

      console.log('Parsed events:', parsedEvents);
      setEvents(parsedEvents);
    } catch (err) {
      console.error('Error fetching transaction history:', err);
      console.error('Error details:', JSON.stringify(err, null, 2));
      setError(err instanceof Error ? err.message : 'Failed to fetch transaction history');
      // Set empty array on error so component can still render
      setEvents([]);
    } finally {
      setIsLoading(false);
    }
  };

  const parseEvent = (
    event: TransactionEvent,
    txHash: string,
    blockHeight: number,
    timestamp: string,
    attrs: Record<string, string>
  ): ParsedEvent | null => {
    // Determine event type from the wasm event type or action attribute
    let eventType: ParsedEvent['type'] = 'unknown';

    if (event.type === 'wasm-deposit' || attrs.method === 'deposit') {
      eventType = 'deposit';
    } else if (event.type === 'wasm-deposit_processed') {
      eventType = 'deposit_processed';
    } else if (event.type === 'wasm' && attrs.method === 'withdraw') {
      eventType = 'withdraw';
    } else if (event.type === 'wasm-price_updated') {
      eventType = 'price_update';
    } else if (event.type === 'wasm-rebalancing_started') {
      eventType = 'rebalancing';
    } else if (event.type === 'wasm' && attrs.method === 'update_prices') {
      // Check if there's a rebalancing event in the same tx
      return null; // We'll catch this via rebalancing_started
    }

    return {
      id: `${txHash}-${event.type}-${blockHeight}`,
      type: eventType,
      timestamp,
      blockHeight,
      txHash,
      data: attrs,
    };
  };

  return {
    events,
    isLoading,
    error,
    refresh: fetchTransactionHistory,
  };
}
