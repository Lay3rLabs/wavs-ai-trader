# Getting Started

This guide will help you set up and run the WAVS vault system on the Neutron-1 mainnet.

## Prerequisites

- Rust and Cargo
- Task (taskfile)
- Docker and Docker Compose
- pnpm (for frontend)

## 1. Environment Setup

First, copy the example environment file and configure your settings:

```bash
cp .env.example .env
```

Edit the `.env` file and set the required fields according to your configuration.

## 2. Start Backend Services

Start all the backend services:

```bash
task backend:start-all OPERATORS=3
```

This will launch the required services for the WAVS vault system.

## 3. Deploy and Configure Services

Since the contracts are already deployed on Neutron-1 mainnet, you can get started on a fresh instance by running these commands:

```bash
# If running locally, in deploy.yml, uncomment the OPERATORS: 3 var

# 1. Set up the service configuration
task deploy:set-service

# 2. Register the service
task deploy:register-service

# 3. Test execution with manual trigger
task deploy:contract-manual-trigger
```

## Configuration Details

The system is currently configured for:
- **Asset Allocation**: Originally 50% USDC, 50% NTRN - Now 25% across all whitelisted assets
- **Rebalancing Interval**: 30 minutes (automatic)
- **Manual Triggering**: Available via `task deploy:contract-manual-trigger`

### Whitelisted Assets

The vault supports these whitelisted assets for deposits:
- **NTRN** (`untrn`) - Neutron native token
- **USDC** (`ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81`) - USD Coin
- **ATOM** (`ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9`) - Cosmos
- **DYDX** (`ibc/2CB87BCE0937B1D1DFCEE79BE4501AAF3C265E923509AEAC410AD85D27F35130`) - dYdX

Only these whitelisted assets can be deposited into the vault. The whitelist can be updated by vault administrators.

## Frontend

To run the frontend:

```bash
cd frontend
pnpm install
pnpm dev
```

## Architecture

The system consists of:
- **Vault Contract**: Manages user deposits and asset allocation
- **Operator**: Handles rebalancing logic and price feeds
- **Aggregator**: Coordinates operations across the network
- **Frontend**: Web interface for user interactions

For more detailed information, see the [Architecture.md](Architecture.md) documentation.