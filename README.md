# WAVS AI Trader

This is an entry for [Hackmos: Cosmoverse 2025](https://dorahacks.io/hackathon/hackmos-2025/detail)

[WAVS](https://www.wavs.xyz) enables us to use [decentralized, verifiable AI agents](https://www.layer.xyz/news-and-insights/deterministic-ai) to actively manage a fund and trade without the risk of a rug.
In conjunction with our bridge this could be done multi-chain, but for now, we are focusing on a PoC of single chain demo.

## Design

* **Open**: Traders can create an AI-managed vault by defining their trading strategies, and deploying to the WAVS operator set for security.
* **Permissionless**: Crypto holders can deposit into the vault in exchange for shares
* **AI Agents**: The AI agents execute the strategies, and trade and invest the tokens in the vault, without any private key that could steal the funds
* **Secure**: A decentralized operator set based on slashable restaked assets runs the agents and must reach consensus to move funds 
* **Liquid**: Holders can withdraw their funds at anytime for their share of the total holdings in the vault
* **Incentivized**: The Vault creator can define a commission they take, either upon withdrawal, or monthly based on AUM.
* **Anti-frontrun**: The planned trades are not visible to the public until WAVS submits a swap transaction to the chain
