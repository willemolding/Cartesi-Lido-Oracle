set dotenv-load

run:
    cargo run --bin orchestrator -- --slot 3612501

build:
    cartesi build

devnet:
    cartesi-coprocessor start-devnet

deploy-devnet:
    cartesi-coprocessor deploy --contract-name <contract name> --network devnet --constructor-args 0xad12e8a0cac7d34fd9987b481fc4ba313a394948a7b5043fada01d858fe8f4ce 0x95401dc811bb5740090279Ba06cfA8fcF6113778