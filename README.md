# Status

This is a proof-of-concept implementation. There are a lot of open TODOs and outstanding optimizations, many of them low hanging fruit. At the current stage testing is carried out mostly manually.

# Usage

More detailed instructions are coming up.

Roughly the steps to benchmark native token transfers are:

1. Bring up a node, see the [`justfile`](./justfile) for details.
2. Execute `just csa` to create accounts.
3. Execute `just bmnf` to run the benchmark.
