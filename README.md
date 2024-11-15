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
  - maybe increase `"produce_chunk_add_transactions_time_limit"`
  - TODO check if `config` adjustments can be done as part of `benchmarknet` modifications
