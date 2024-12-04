# Status

This is a proof-of-concept implementation. There are a lot of open TODOs and outstanding optimizations, many of them low hanging fruit. At the current stage testing is carried out mostly manually.

# Usage

More detailed instructions are coming up.

Roughly the steps to benchmark native token transfers are:

1. Bring up a node, see the [`justfile`](./justfile) for details.
2. Execute `just csa` to create accounts.
3. Execute `just bmnf` to run the benchmark.

Note that OS default limits on the number of file descriptors might be too tight. The limit can be queried with `ulimit -n` and it should be larger than the value passed to `--channel-buffer-size`. Roughly speaking because each connection to the RPC requires a file descriptor. The limit can be increased with `ulimit -n <number>`. Exceeding the limit results in `Too many open file` errors.

# Unlimited config

- Modify `genesis.json`:
  - `"chain_id": "benchmarknet"`
    - TODO sent PR to patch benchmarknet config
  - increase `gas_limit`
- Modify `config.json`
  - set `"load_mem_tries_for_tracked_shards": true,`
  - increase `"produce_chunk_add_transactions_time_limit"`
  - increase `view_client_threads`
  - TODO check if `config` adjustments can be done as part of `benchmarknet` modifications
- Code changes
  - Ensure [`benchmarknet` adjustments](https://github.com/near/nearcore/blob/1324fe938cd840de99a4eb5ff57a301fad085d1a/core/parameters/src/config_store.rs#L147) are up to date.
  - Maybe increase the number of RPC workers [here](https://near.zulipchat.com/#narrow/channel/308695-nearone.2Fprivate/topic/native.20token.20transfer.20benchmark/near/485901127)
