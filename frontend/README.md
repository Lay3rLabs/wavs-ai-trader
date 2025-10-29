# WAVS AI Trader Frontend

A modern Next.js frontend for the WAVS AI-managed trading vault.

## Features

- **Wallet Integration**: Connect with Keplr wallet
- **Deposit Assets**: Deposit whitelisted tokens into the vault
- **Withdraw Shares**: Withdraw your proportional share of vault assets
- **Real-time Stats**: View vault TVL, holdings, and price feeds
- **Pending Deposits**: Track deposits awaiting price updates
- **Modern UI**: Vercel-inspired design with Tailwind CSS

## Setup

1. Install dependencies:

   ```bash
   pnpm install
   ```

2. Create a `.env.local` file with your configuration:

   ```bash
   cp .env.example .env.local
   ```

3. Update the environment variables:

   - `NEXT_PUBLIC_CHAIN_ID`: The chain ID (e.g., `neutron-1`)
   - `NEXT_PUBLIC_RPC_ENDPOINT`: RPC endpoint URL
   - `NEXT_PUBLIC_VAULT_CONTRACT_ADDRESS`: Your deployed vault contract address

4. Run the development server:

   ```bash
   pnpm dev
   ```

5. Open [http://localhost:3000](http://localhost:3000) in your browser

## How It Works

### Deposit Flow

1. Connect your Keplr wallet
2. Select a whitelisted token and enter amount
3. Submit deposit transaction
4. Deposit enters "pending" state
5. When WAVS AI agents update prices, your deposit is processed and shares are issued

### Withdraw Flow

1. Enter the number of shares to withdraw
2. Submit withdrawal transaction
3. Receive proportional share of all vault assets

### Vault Stats

- **Total Value Locked (TVL)**: Total USD value of all assets
- **Active Assets**: Number of different token types held
- **Pending Deposits**: Deposits awaiting price updates
- **Current Prices**: Active price feeds from WAVS AI

## Contract Bindings

The `src/contract-bindings` directory contains TypeScript bindings generated from the CosmWasm contract schemas. These provide type-safe interfaces for interacting with the vault contract.
