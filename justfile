near_localnet_home := ".near-localnet-home/"
near_sandbox_home := ".near-sandbox-home/"
rpc_url := "http://127.0.0.1:3030"

# `near-sandbox` binary can be downloaded or built by running `make sandbox` in `nearcore`.
init_sandbox:
    ./neard-sandbox --home {{near_sandbox_home}} init

run_sandbox:
    ./neard-sandbox --home {{near_sandbox_home}} run

# After this, you might want to:
# - enable memtries: `load_mem_tries_for_tracked_shards: true` in `config.json`
# - increase test.near's balance in genesis.json.
#   - max amount before overflow: 1000000000000000000000000000000000
init_localnet:
    ./neard --home {{near_localnet_home}} init --chain-id localnet

run_localnet:
    ./neard --home {{near_localnet_home}} run

# Deposit should cover at least 10 transfers of 1.
csa:
    RUST_LOG=info \
    cargo run -p cmd --release -- create-sub-accounts \
        --rpc-url {{rpc_url}} \
        --signer-key-path {{near_localnet_home}}/validator_key.json \
        --nonce 1 \
        --sub-account-prefix 'a' \
        --num-sub-accounts 10000 \
        --deposit 953060601875000000010000 \
        --channel-buffer-size 1200 \
        --interval-duration-micros 1500 \
        --user-data-dir user-data

ccreate:
    cargo run -p cmd --release -- create-contract \
        --rpc-url "http://localhost:3030" \
        --signer-key-path {{near_localnet_home}}/validator_key.json \
        --nonce 1772 \
        --deposit 17697099999999999980000000 \
        --new-account-id ft1.test.near \
        --wasm-path assets/fungible_token.wasm \
        --user-data-dir contract-data

# Avoid attaching excessive gas.
# --args '{"owner_id": "{{receiver_id}}", "total_supply": "1000000000000000", "metadata": { "spec": "ft-1.0.0", "name": "Example Token Name", "symbol": "EXLT", "decimals": 8 }}' \
ccall receiver_id:
    cargo run -p cmd --release -- call-contract \
        --rpc-url "http://localhost:3030" \
        --signer-key-path contract-data/{{receiver_id}}.json \
        --nonce 6737000006 \
        --receiver-id {{receiver_id}} \
        --method-name new_default_meta \
        --args '{"owner_id": "{{receiver_id}}", "total_supply": "10000000000000000"}' \
        --gas 100000000000000 \
        --deposit 0

# Seems like current max is ~2400 native transfers per second.
# Set a `interval-duration-micros` to roughly sent transactions at that rate.
bmnf:
    RUST_LOG=info \
    cargo run -p cmd --release -- benchmark-native-transfers \
        --rpc-url {{rpc_url}} \
        --user-data-dir user-data/ \
        --num-transfers 20000 \
        --interval-duration-micros 400 \
        --amount 1

view_account id:
    http post {{rpc_url}} jsonrpc=2.0 id=dontcare method=query \
        params:='{ \
            "request_type": "view_account", \
            "finality": "optimistic", \
            "account_id": "{{id}}" \
        }'

view_keys id:
    http post {{rpc_url}} jsonrpc=2.0 id=dontcare method=query \
        params:='{ \
            "request_type": "view_access_key_list", \
            "finality": "final", \
            "account_id": "{{id}}" \
        }'

view_protocol_config:
    http post {{rpc_url}} jsonrpc=2.0 id=dontcare method=EXPERIMENTAL_protocol_config \
      params:='{ \
        "finality": "final" \
      }'
