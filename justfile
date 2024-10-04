near_localnet_home := ".near-localnet-home/"
near_sandbox_home := ".near-sandbox-home/"
rpc_url := "http://localhost:3030"

# `near-sandbox` binary can be downloaded or built by running `make sandbox` in `nearcore`.
# After this, you might want to increase test.near's balance in genesis.json.
init_sandbox:
    ./neard-sandbox --home {{near_sandbox_home}} init

run_sandbox:
    ./neard-sandbox --home {{near_sandbox_home}} run

init_localnet:
    ./neard --home {{near_localnet_home}} init --chain-id localnet

run_localnet:
    ./neard --home {{near_localnet_home}} run

# Deposit should cover at least 1000 transfers of 1.
csa:
    cargo run -p cmd -- create-sub-accounts \
        --rpc-url "http://localhost:3030" \
        --signer-key-path {{near_localnet_home}}/validator_key.json \
        --nonce 2022 \
        --num-sub-accounts 5 \
        --deposit 55306060187500000001000 \
        --user-data-dir user-data

ccreate:
    cargo run -p cmd -- create-contract \
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
    cargo run -p cmd -- call-contract \
        --rpc-url "http://localhost:3030" \
        --signer-key-path contract-data/{{receiver_id}}.json \
        --nonce 6737000006 \
        --receiver-id {{receiver_id}} \
        --method-name new_default_meta \
        --args '{"owner_id": "{{receiver_id}}", "total_supply": "10000000000000000"}' \
        --gas 100000000000000 \
        --deposit 0

bmnf:
    cargo run -p cmd -- benchmark-native-transfers \
        --rpc-url "http://localhost:3030" \
        --user-data-dir user-data/

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
