# JSON-RPC samples

Real traffic, not made up: captured from Ethereum mainnet (chain 1, block
26393566 = `0x192bbde`) through `https://ethereum-rpc.publicnode.com` on
2026-11-12, and kept byte for byte but minified. Public chain data, so there
is nothing in here to keep secret.

`*.req.json` is what a client sends, `*.res.json` what came back.

| File | What it is for |
|---|---|
| `batch` | Three calls in one request: the `+2` in a method name |
| `call_balanceof` | `eth_call` of ERC-20 `balanceOf`: selector `0x70a08231`, result 39.192124 USDC |
| `send_raw` | A real signed EIP-1559 transaction, 884 bytes of hex to decode |
| `tx.res.json` | The same transaction as the node sees it, to check a decoder against |
| `send_raw_legacy` / `tx_legacy.res.json` | The same pair for a transaction of the kind from before EIP-2718, whose `v` carries the chain (EIP-155) |
| `revert` | A reverted `eth_call`: **HTTP 200**, code `3`, `Error(string)` data |
| `error_no_method` | The plain JSON-RPC `-32601` |
| `get_logs` | Logs with `Transfer` topics; cut from 67 logs to 3 |
| `receipt.res.json` | A receipt, cut from 11 logs to 2 |
| `mcp.req.json` | MCP over HTTP: JSON-RPC too, and not a chain call |
| `multicall.req.json` | A real Multicall3 `aggregate3`: 4,772 bytes of calldata, tuples inside an array inside a call |
