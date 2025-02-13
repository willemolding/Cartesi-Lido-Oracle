set dotenv-load

build:
    cartesi build

devnet:
    cartesi-coprocessor start-devnet

deploy-devnet:
    #!/usr/bin/env bash
    output=$(cartesi-coprocessor address-book)

    MACHINE_HASH=$(echo "$output" | grep "Machine Hash" | awk '{print $3}')
    DEVNET_TASK_ISSUER=$(echo "$output" | grep "Devnet_task_issuer" | awk '{print $2}')

    cartesi-coprocessor deploy --contract-name CartesiLidoOracle --network devnet --constructor-args $DEVNET_TASK_ISSUER $MACHINE_HASH

run-task slot:
    RUST_LOG=debug cargo run --bin orchestrator -- --slot {{slot}}