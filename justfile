set dotenv-load

build:
    cartesi build

devnet-up:
    cartesi-coprocessor start-devnet

devnet-down:
    cartesi-coprocessor stop-devnet

deploy-devnet:
    #!/usr/bin/env bash
    output=$(cartesi-coprocessor address-book)

    MACHINE_HASH=$(echo "$output" | grep "Machine Hash" | awk '{print $3}')
    DEVNET_TASK_ISSUER=$(echo "$output" | grep "Devnet_task_issuer" | awk '{print $2}')

    cartesi-coprocessor deploy --contract-name CartesiLidoOracle --network devnet --constructor-args $DEVNET_TASK_ISSUER $MACHINE_HASH

deploy-holesky:
    #!/usr/bin/env bash
    output=$(cartesi-coprocessor address-book)

    MACHINE_HASH=$(echo "$output" | grep "Machine Hash" | awk '{print $3}')
    DEVNET_TASK_ISSUER=$(echo "$output" | grep "Devnet_task_issuer" | awk '{print $2}')

    cartesi-coprocessor deploy -p $ETH_PRIVATE_KEY -r $ETH_RPC_URL --contract-name CartesiLidoOracle --network testnet --constructor-args $DEVNET_TASK_ISSUER $MACHINE_HASH

trigger-oracle slot:
    RUST_LOG=orchestrator=debug cargo run --release --bin orchestrator -- --slot {{slot}}

## Manually running

carize:
    docker run --rm \
        -v $(pwd)/.cartesi/image:/data \
        -v $(pwd):/output \
        ghcr.io/zippiehq/cartesi-carize:latest /carize.sh

upload-to-solver:
    #!/usr/bin/env bash    

    curl -X POST -F "file=@output.car" "http://localhost:5001/api/v0/dag/import"

    CID=$(cat output.cid)
    SIZE=$(cat output.size)
    MACHINE_HASH=$(xxd -p .cartesi/image/hash | tr -d '\n')

    while true; do
        result=$(curl -X POST "$SOLVER_URL/ensure/$CID/$MACHINE_HASH/$SIZE")
        echo "$result"
        if [[ "$result" == *"ready"* ]]; then
            break
        fi
        sleep 3
    done
    
deploy-contracts:
    #!/usr/bin/env bash
    output=$(cartesi-coprocessor address-book)
    MACHINE_HASH=$(echo "$output" | grep "Machine Hash" | awk '{print $3}')
    DEVNET_TASK_ISSUER=$(echo "$output" | grep "Devnet_task_issuer" | awk '{print $2}')

    cd contracts
    forge create --broadcast \
      --rpc-url $ETH_RPC_URL \
      --private-key $ETH_PRIVATE_KEY \
      ./src/CartesiLidoOracle.sol:CartesiLidoOracle \
      --constructor-args $DEVNET_TASK_ISSUER $MACHINE_HASH

deploy-contracts-and-verify:
    #!/usr/bin/env bash
    output=$(cartesi-coprocessor address-book)
    MACHINE_HASH=$(echo "$output" | grep "Machine Hash" | awk '{print $3}')
    DEVNET_TASK_ISSUER=$(echo "$output" | grep "Devnet_task_issuer" | awk '{print $2}')

    cd contracts
    forge create --broadcast \
      --rpc-url $ETH_RPC_URL \
      --private-key $ETH_PRIVATE_KEY \
      --verify \
      --etherscan-api-key $ETHERSCAN_API_KEY \
      --verifier-url $ETHERSCAN_API_URL \
      ./src/CartesiLidoOracle.sol:CartesiLidoOracle \
      --constructor-args $DEVNET_TASK_ISSUER $MACHINE_HASH

## Testing with nonodox

start-anvil:
    anvil --load-state anvil_state.json

# Start the coprocessor program in the Cartesi machine emulator.
# Have `nonodox` running first or this will error
start-machine:
    cartesi-machine --network --flash-drive=label:root,filename:.cartesi/image.ext2 --env=ROLLUP_HTTP_SERVER_URL=http://10.0.2.2:5004 -- /opt/cartesi/dapp/dapp


