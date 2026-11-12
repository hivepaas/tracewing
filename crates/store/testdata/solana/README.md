# Solana samples

Real traffic, not made up: captured from Solana mainnet-beta through
`https://api.mainnet-beta.solana.com` on 2026-11-12, kept byte for byte but
minified. Public chain data.

| File | What it is for |
|---|---|
| `send_transaction` | What a client sends to put a transaction on the chain: one base64 string, 660 characters |
| `transaction.res.json` | **The same transaction as the node reads it**, to check a decoder against |
| `send_transaction_v1.req.json` | A transaction the node calls version 1, whose bytes are laid out by SIMD-0385: fixed-width counts, a config mask, and the signature at the end rather than the front |
| `transaction_v1.res.json` | The same one as the node reads it |
| `balance` | Lamports, which are what SOL is counted in |
| `account_info` | An account with its data as base64, owned by the token program |
| `version_error.res.json` | What a node says when a client asks for a transaction newer than it will read |
