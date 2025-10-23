# WAVS AI Trader - Plan

## Goal
Build a minimal AI-trading vault

## Chain Strategy
- **Home Chain**: Neutron (permissionless environment)
- **DEX**: Astroport (native to Neutron, permissionless)
- **Target Assets**: Start with high-liquidity assets on Astroport

## Core Components

### 1. Vault Contract
- CosmWasm smart contract for deposit/withdrawal
- Share-based accounting system
- Multi-operator control (2/3 threshold via WAVS)
- Astroport integration for swaps

### 2. Share System
- Calculate share value based on total vault holdings
- Price feed integration for accurate valuation
- Support for multi-asset deposits and withdrawals
- Proportional distribution of vault assets

### 3. Trading Strategy
- Simple starting strategy (50/50 split between two assets)
- AI agent integration for strategy execution
- Batch epoch processing for rebalancing
- Market condition analysis for target allocation

### 4. Cross-Chain Support
- IBC money routing from other chains to Neutron
- Asset bridging strategies
- Multi-chain vault synchronization

## Implementation Steps

### Step 1: Vault Contract Development
**Core functionality:**
- Accept deposits of whitelisted tokens
- Issue shares based on deposit value
- Allow withdrawal of shares for proportional assets
- Only allow trades signed by 2/3 of operators

### Step 2: Share Logic Implementation
**Price normalization:**
- Use price feeds to calculate total vault value
- Convert any deposited token to USD value
- Issue shares based on USD value at time of deposit
- Maintain share-to-asset ratio calculations

**Example:**
- Vault holds: 1000 ATOM ($8,000) + 2000 NTRN ($2,000) = $10,000 total
- User deposits 100 ATOM ($800)
- User receives 8% of total shares (800/10000)

### Step 3: Astroport Integration
**Swap functionality:**
- Direct integration with Astroport contracts

### Step 4: Basic UI
**Simple interface:**
- Connect wallet (Keplr)
- View vault holdings and share price
- Deposit tokens
- Withdraw shares
- Basic portfolio view

### Step 5: AI
**Trading Strategy:**
- AI agent analyzes market conditions
- Determines target allocation (e.g., 70% ATOM, 30% NTRN)

## Asset Selection Strategy

**Initial Assets (high liquidity on Astroport):**
- ATOM
- NTRN (native token)
- USDC
- wBTC
- TIA

**Selection criteria:**
- Deep liquidity on Astroport
- Reliable price feeds
- Low volatility for initial testing
- User familiarity and demand

---

*Focus on delivering core functionality with room for future expansion.*