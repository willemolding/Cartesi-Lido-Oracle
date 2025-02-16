# Cartesi Lido Oracle

A coprocessor that enhances the Lido protocol by replacing trusted parties with provable computation. Built for Cartesi+Eigenlayer Experiment Week 2025.

## About

To correctly rebase stETH Lido needs to know the total staked ETH held by all Lido validators. This information lives in the beacon state which inaccessible to smart contracts. Even if it was the gas required to iterate over all the >1M validators makes it infeasible.

Lido currently gets around this by having trusted offchain actors compute this value and pass it to the protocol via a 5-of-9 multisig. But they want to do better! There is currently an open proposal (LIP-23) for submissions to build trustless backstops for these oracles. So far there have been a number attempts to solve this using ZK with mixed success. Of 5 proposals awarded grants to build ZK oracles only 2 have succeeded. The main challenge being the size of the validator set.

This project gives an alternative solution using Cartesi Coprocessors and Eigenlayer. It solves a real and current problem faced by the largest DeFi protocol in the world. After this hackathon I hope it will be considered as a candidate for a further Lido grant.



## How it works

With [EIP-4788](https://eips.ethereum.org/EIPS/eip-4788) a trusted beacon block root can be obtained in the Ethereum execution layer. This is used as the input to the coprocessor along with another hash used to bootstrap preimage data retrieval.

The beacon block and beacon state are loaded into the coprocessor via the Cartesi preimage oracle and verified against the trusted block root. The validator data in the beacon state is used to derive the relevant values for the Lido oracle which forms the coprocessor output. As specified in [LIP-23](https://github.com/lidofinance/lido-improvement-proposals/blob/develop/LIPS/lip-23.md) the data required for a rebase is:

```
OracleReport {
    clBalanceGwei, // the total balance held by all Lido validators
	withdrawalVaultBalanceWei, // balance held in the withdrawal vault
	totalDepositedValidators, // total number of Lido validaotrs
	totalExitedValidators, // number of Lido validators that have exited
}
```

3/4 of these values can be calculated by iterating over the beacon state. Lido validators can be identified by their withdrawal credentials. The `withdrawalVaultBalanceWei` must be calculated from execution state and is outside the scope of this project for now.

This report is received back on-chain via a staked operator and can then be consumed by the Lido protocol via the LIP-23 second opinion oracle interface.

## Running the Demo

### Prerequisites

To run the demo please install:

- [Rust dev tools](https://www.rust-lang.org/tools/install)
- All prerequisites from the [Cartesi coprocessor tutorial](https://docs.mugen.builders/cartesi-co-processor-tutorial/installation)
- [just](https://github.com/casey/just)

### Local Devnet

In one shell start the local devnet with

```shell
just devnet-up
```

Ensure the `.env` file has the required fields populated for the devnet. See [.env.example]. A beacon RPC can be obtained from [Quicknode](https://www.quicknode.com/).

In another begin by building the coprocessor program, car-izing it, and uploading it to the devnet IPFS node so it is available to the operator

```shell
just build
just carize
just upload-to-solver
```

Deploy the contracts with the correct machine hash derived from above

```shell
just deploy-contracts
```

Paste the `deployed to` contract address into the `.env` file before continuing

Use the orchestrator CLI to trigger an oracle report for the given beacon slot (e.g. 3647904)

```shell
just trigger-oracle 3647904
```

This will:

- Download the beacon state for the given slot
- Split it into chunks small enough for the preimage oracle and upload them to the operator
- Submit a transaction to the contract to request an oracle report from the coprocessor

> [!IMPORTANT]  
> It will take quite a while for the coprocessor to complete the request as the beacon state can be pretty large (>100MB)

Since there is no beacon blocks for the devnet chain this calls the `generateReportUntrusted` function which skips obtaining a trusted beacon root in the contract via EIP-4788. !This is for testing only!

## Holesky Testnet

Configure the [.env] file with an RPC for Holesky as well as an API URL and key for verifying the contract. Ensure the private key is funded with Holesky Eth. Obtain the URL of an operator for Holesky (hopefully this will become easier in the future)

Deploy the coprocessor with:

```shell
just deploy-holesky
```

Deploy the contracts again so they are verified and can be viewed nicely on etherscan

```shell
just deploy-contracts-and-verify
```

Trigger an oracle report with

```shell
just trigger-oracle 3647904
```

## Challenges Faced

What I struggled most with was the missing documentation or examples for machine IO and the integration of that with the coprocessor operator Figuring out how to do this properly probably took longer than the rest of the project combined.

Another main challenge was a slow dev loop. I had even less luck using machine IO with Nonodox (it may be possible but something else undocumented) and so I was testing everything against the devnet. This was made worse by the fact that the operator didn't seem to recover properly from some errors requiring the devnet to be restarted which was a slow process.

I anticipate that that most projects that want to use a coprocessor to do large computations will also require large inputs and so supporting preimage IO and tooling around it should be a high priority.

## Future Improvements

It is pretty simple to reduce the amount of data that the coprocessor needs to load by compressing unused fields in the beacon state into their hashed form. Thanks to the SSZ hash_tree_root algorithm this will not change the root has and so we can still match it with the hash obtained on-chain. This was skipped in the hackathon for implementation simplicity.
