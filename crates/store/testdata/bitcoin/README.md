# Bitcoin samples

The transactions are real, from Bitcoin mainnet block 975534, taken from
blockstream.info's public API on 2026-11-12: `*.req.json` holds the raw hex a
client would send, `*.tx.json` the same transaction as that API reads it, which
is what the decoder is checked against — its txid, its values in satoshis, each
output's kind and the address it pays.

The JSON-RPC envelope around the hex is written here rather than captured:
Bitcoin Core's RPC wants credentials, so there is no public node to record. The
shape is `sendrawtransaction` as Bitcoin Core documents it, and the hex inside
it is untouched.

| File | What it is for |
|---|---|
| `segwit` | Two bech32 outputs (`v0_p2wpkh`), the common shape today |
| `legacy_and_script` | A base58 address (`p2pkh`) beside a `v0_p2wsh` |
| `taproot` | A bech32m address (`v1_p2tr`), which checksums differently |
| `p2sh` | A base58 address that starts `3` |
