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
  - QUESTION: Is this querying the pools on neutron? or an external oracle?
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
- Use price feeds to calculate total vault value (can be simple spot price on Astroport pool for now, see step 6)
- Convert any deposited token to USD value (maintain whitelist of which tokens can be depositied, not everything)
- Issue shares based on USD value at time of deposit
- Maintain share-to-asset ratio calculations

**Example:**
- Vault holds: 1000 ATOM ($8,000) + 2000 NTRN ($2,000) = $10,000 total
- User deposits 100 ATOM ($800)
- User receives 8% of total shares (800/10000)

### Step 3: Astroport Integration
**Swap functionality:**
- Direct integration with Astroport contracts
- Trigger epochs in WAVS and determine trading strategy there
- WAVS code (Rust) does analysis from market conditions and current allocation and decides on desired allocation
- Vault contract queries exact spot price on Astroport pool for each token at time of trade and converts the allocation request into swaps (prevent drift from price changes)
### Step 4: Basic UI
**Simple interface:**
- Connect wallet (Keplr)
- View vault holdings and share price
  - Bonus: historical share price
- Deposit tokens
- Withdraw shares
- Basic portfolio view
    - Bonus: historical token holdings

### Step 5: AI
**Trading Strategy:**
- AI agent analyzes market conditions
- Determines target allocation (e.g., 70% ATOM, 30% NTRN)

### Step 6: Productionize
- Provide some slippage and price checks on swaps
  - If there is a frontrun between calculation and execution, the trade aborts rather than trade at a bad price 
- Make robust price feeds using weighted average over last hour, demonstrates oracles

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

**Issue:** We want to deploy outpost contracts on external chains. In this set, we can deposit all of them on Neutron **after** briding them. How do we manage this?
1. Look for other chains with permissionless CosmWasm support and pools on Neutron
2. Try to get hackathon contract through Cosmos Hub governance (lol)
3. Just use normal IBC20 messages from the output chains (design issue)

## IBC Support

The original idea was CosmWasm (or Solidity) contracts on the outpost chains. This doesn't seem to be practical here, so let's explore what else we can do with WAVS.
The user should be able to interact with the UI and only sign transactions on a remote chain. This will be an IBC transfer to the vault contract, possibly filling information in the memo field.

Deposit:
- User signs IBC20 transaction on Cosmos Hub to transfer tokens to Neutron
- Upon receipt, WAVS calculates shares as normal local deposit (either IBC hook, or WAVS watches for event and triggers)
- "Owner" is marked as remote address, no translation of address

Withdrawal:

- When the user wants to withdraw the shares, they sign another IBC transfer from the Cosmos Hub to the vault contract on Neutron
- This has a very low token value (anything above 0 to be valid transfer)
- It includes a message in the memo field, which is parsed by the vault contract to determine the amount of shares to withdraw
- This triggers the vault contract to convert shares into the one remote token (ATOM) based on value
- The vault contract then starts an IBC transfer to send the tokens to the user

Note: A future iteration could make this a bit less tied to the IBC memos, and handle transfers initiated from Ethereum, Solana, etc. Even if they pass via Wormhole, Axelar, etc

---

*Focus on delivering core functionality with room for future expansion.*