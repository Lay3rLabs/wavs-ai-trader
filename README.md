# WAVS AI Trader

This is an entry for [Hackmos: Cosmoverse 2025](https://dorahacks.io/hackathon/hackmos-2025/detail)

[WAVS](https://www.wavs.xyz) enables us to use [decentralized, verifiable AI agents](https://www.layer.xyz/news-and-insights/deterministic-ai) to actively manage a fund and trade without the risk of a rug.
In conjunction with our bridge this could be done multi-chain, but for now, we are focusing on a PoC of single chain demo.

## Problem Space

The explosion of DeFi primitives has given active traders numerous possibilities for deploying capital, from risk-averse strategies to high-risk, high-reward approaches—from staking tokens to MEV extraction with flash loans. However, many crypto holders lack the skill or time to manage their money effectively.

They would benefit from "managed funds" operated by skilled traders who can react to changing market conditions to seek better yields. One approach is on-chain managed funds like Yearn vaults, which provide easy access to complex strategies and security against hacks, but these remain relatively static strategies whose values fluctuate over time. Another solution is entrusting tokens to someone to trade on your behalf—they can actively adjust strategies based on market and macroeconomic sentiment, but this requires a very high level of trust and ideally legal enforcement mechanisms.

Leveraging recent advances in off-chain verifiable services, we provide a novel middle ground: actively managed vaults secured by a PoS operator set and powered by AI agents.
Skilled traders can extract their approaches into a programmable strategy, leveraging both mathematical analysis and LLM analysis of large market trends.
This strategy is then executed by AI agents, which are run by many operators simultaneously and require a 2/3 threshold to move funds, ensuring no party can rug the protocol.

This approach allows us to build something as secure and easy to use as a Yearn vault, but with a large active component of AI agents that dynamically balance funds and adapt strategies based on changing market conditions.

## Design

* **Open**: Traders can create an AI-managed vault by defining their trading strategies, and deploying to the WAVS operator set for security.
* **Permissionless**: Crypto holders can deposit into the vault in exchange for shares
* **AI Agents**: The AI agents execute strategies and trade/invest the vault tokens without any private key that could steal funds
* **Secure**: A decentralized operator set based on slashable restaked assets runs the agents and must reach consensus to move funds 
* **Liquid**: Holders can withdraw their funds at any time for their share of the total holdings in the vault, or place their shares in a liquidity pool or lending protocol
* **Incentivized**: The Vault creator can define a commission they take, either upon withdrawal or monthly based on AUM
* **Anti-frontrun**: The planned trades are not visible to the public until WAVS submits a swap transaction to the chain

## Implementation

There are four main components in the system:

* **DeFi primitives** - Existing DeFi protocols on the chain where the trading vault is deployed
* **Vault** - A (Solidity/CosmWasm) contract deployed to the chain. Each trader has their own vault contract, but they can likely use the same code with different configurations of the service that controls the vault
* **Trading Strategy** - A WAVS component that defines the trading strategy for the vault. It can query web2 and web3 sources and deterministically execute LLMs. This encapsulates the trader's knowledge and makes the vault active and attractive for investment
* **WAVS Operator Set** - A set of operators responsible for executing the WAVS service and collectively managing the vault according to instructions

The blockchain and DeFi primitives already exist—we only need to integrate with them. The WAVS operator set is standard infrastructure we can deploy. This leaves the vault, trading strategy, and a UI to interact with the vault as the core components to develop. This is covered in more detail in [Architecture](./docs/Architecture.md), but here is an overview:

### Vault

The vault manages various tokens—both native tokens and tokens from other protocols (like LP shares or liquid staking tokens). It is aware of the total value of its holdings and the operator set that controls it. It has no knowledge of trading strategies. Once configured, it allows these operations:

* **Balanced Deposit** - Any user can deposit tokens into the vault to receive shares. The deposited tokens must reflect the current value of the vault in terms of token mix
* **Withdraw** - Any user can return shares to receive their portion of all tokens managed by the vault (e.g., if you return 1% of shares, you get 1% of each token held by the vault)
* **Trade** - Only a message signed by 2/3 of the operator set can perform this operation. It executes trading strategies to rebalance the vault's holdings to the target token mix. This is also used by "Auto Deposit" and "Auto Withdraw" to convert user funds

### Stretch Goals

* **Auto Deposit** - Deposit only one token (from a whitelisted set) for a minimum number of shares. The trader analyzes how to swap this token, and if possible, creates a trade to balance the vault and mint shares to the depositor. If the trade isn't possible, the deposit is returned to the depositor
* **Auto Withdraw** - Request withdrawal in a single token (likely the chain's native token or USDC). The trader is responsible for unwinding complex strategies (like selling LP shares, exiting lending markets, etc.). This is simpler for users who don't want to manage multiple strategies but focuses on immediate liquidation rather than maximum return


### Trading Strategy

TODO

## Future Work

Since WAVS unifies on-chain, off-chain, and cross-chain projects, extending this to a cross-chain vault that leverages IBC and seeks strategies across multiple chains should be relatively straightforward.

While there isn't sufficient time during the Hackathon for this implementation, we'd love to add it as a follow-up. The hardest part would likely be depositing and withdrawing shares, as there's no longer a way to atomically snapshot the value of one share across multiple vaults on different chains. We're happy to discuss approaches with anyone interested.