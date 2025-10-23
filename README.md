# WAVS AI Trader

This is an entry for [Hackmos: Cosmoverse 2025](https://dorahacks.io/hackathon/hackmos-2025/detail)

[WAVS](https://www.wavs.xyz) enables us to use [decentralized, verifiable AI agents](https://www.layer.xyz/news-and-insights/deterministic-ai) to actively manage a fund and trade without the risk of a rug.
In conjunction with our bridge this could be done multi-chain, but for now, we are focusing on a PoC of single chain demo.

## Problem Space

The explosion of DeFi primitives has allowed active traders a large number of possibilities to use their money, from risk-averse strategies to high risk, high reward strategies.
From staking your token vs trying to MEV with flash loans. However, a large number of crypto holders don't have the skill or the time to manage their money well.
They would benefit from "managed funds" that are managed by a skilled trader and can react to changing market conditions to seek out better yields.
One approach to solve this problem is on-chain managed funds, like Yearn vaults, which provide easy access to complex strategies, and provide security against hacks,
but they are still relatively static strategies, whose value fluxuates over time. Another solution is giving your tokens to someone to trade for you - they can actively
adjust strategies based on market and macroeconomic sentiment, but this requires a very high level of trust, and ideally legal enforcement mechanisms.

Leveraging recent advances in off-chain verifiable services, we provide a novel middleground: actively managed vaults secured by a PoS operator set, and powered by AI agents.
Skilled traders can extract their approaches into a programmable strategy, leveraging both mathematical analysis as well as LLM analyis of largr market trends.
This strategy is then executed by AI agents, which are executed by many operators simulateously and requiring a 2/3 threshold to move funds, to ensure no party can rug the protocol.

This approach allows us to build something as secure and easy to use as a Yearn vault, but with a large active component of AI agents that dynamically balance funds and adapt strategies based on changing market conditions.

## Design

* **Open**: Traders can create an AI-managed vault by defining their trading strategies, and deploying to the WAVS operator set for security.
* **Permissionless**: Crypto holders can deposit into the vault in exchange for shares
* **AI Agents**: The AI agents execute the strategies, and trade and invest the tokens in the vault, without any private key that could steal the funds
* **Secure**: A decentralized operator set based on slashable restaked assets runs the agents and must reach consensus to move funds 
* **Liquid**: Holders can withdraw their funds at anytime for their share of the total holdings in the vault, or place their shares in a liquidity pool or lending protocol
* **Incentivized**: The Vault creator can define a commission they take, either upon withdrawal, or monthly based on AUM.
* **Anti-frontrun**: The planned trades are not visible to the public until WAVS submits a swap transaction to the chain

## Implementation

There are four main components in the system:

* DeFi primitives - These are existng DeFi protocols on the chain that the trading vault is deployed on.
* Vault - This is a (Solidity / CosmWasm) contract that is deployed to the chain. Each trader has their own vault contract, but they can most likely use the same code, just different configurations of the service that controls the vault.
* Trading Strategy - This is a WAVS component that defines the trading strategy for the vault. It can query web2 and web3 sources and deterministically execute LLMs as well. This encapsulates the trader's knowledge and makes the vault active and interesting to invest in.
* WAVS Operator Set - This is a set of operators that are responsible for executing the WAVS service, and who collectively manage the vault according to the instructions

The blockchain itself and the DeFi primitives already exist, we only look to integrate with them. And the WAVS operator set is standard infrastructure we can deploy. This leaves the vault and the trading strategy as the core components to code. As well as a UI to interact with the vault. This is covered in more detail in [Architecture](./docs/Architecture.md), but here is an overview:

### Vault

The vault manages a number of tokens - both native tokens as well as tokens in other protocols (like LP shares, or liquid staking tokens). It is aware of the total value of its holdings and the operator set that controls it. It has no knowledge of trading strategies. Once configured, it allows a few different operations:

* Balanced Deposit - Any user can deposit tokens into the vault to get a certain number of shares. The tokens deposited must reflect the current value of the vault, in terms of mix of tokens.
* Withdraw - Any user can return a number of shares to receive their portion of all tokens managed by the vault. eg. if you return 1% of the shares, you get 1% of each token held by the vault. 
* Trade - Only a message signed by 2/3 of the operator set can perform this operation. It executes any trading strategy to rebalance the vault's holdings to the target mix of tokens. This is also used by "Auto Deposit" and "Auto Withdraw" in order to convert users funds as well.

Stretch Goals:

* Auto Deposit - Another approach is to deposit only one token (out of a set of whitelisted tokens), and a minimum number of shares to receive. The trader will analyze how to swap this, and if possible, will create a trade to balance it and mint shares to the depositor. If the trade is not possible, the deposit will be returned to the depositor. 
* Auto Withdraw - Another approach is to ask the vault to provide you with your withdrawl in one token (the chain native token or USDC most likely). The trader will then be responsible fro unrolling the complex strategies (like selling LP shares, pulling out of lending markets, etc). This is simpler for users who don't want to manage multiple strategies but focuses on immediate liquidation rather than maximum return.


### Trading Strategy

TODO

## Future Work

Since WAVS unifies on-chain, off-chain, and cross-chain projects, it should be relatively easy to move this to a cross-chain vault, that is able to leverage IBC and seek strategies on multiple chains.
There is not sufficient time in the Hackathon for it, but we would love to add that as a follow-up. The hardest part would probably be depositing and withdrawing the shares, as there is no longer a way to atomicly snapshot the value of one share over the multiple vaults on multiple chains. Happy to discuss approaches with anyone else here.