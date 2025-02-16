# Cartesi Lido Oracle

A coprocessor that enhances the Lido protocol by replacing trusted parties with provable computation. Built for Cartesi+Eigenlayer Experiment Week 2025!

## About

To correctly rebase stETH Lido needs to know the total staked ETH held by all Lido validators. This information lives in the beacon state which inaccessible to smart contracts. Even if it was the gas required to iterate over all the >1M validators on mainnet makes it infeasible.

Lido currently gets around this by having trusted offchain actors compute this value and pass it to the protocol via a 5-of-9 multisig. But they want to do better! There is currently an open proposal (LIP-23) for submissions to build trustless backstops for these oracles. So far there have been a number attempts to solve this using ZK with mixed success. Of 5 proposals awarded grants to build ZK oracles only 2 have succeeded. The main challenge being the size of the validator set.

This project gives an alternative solution using Cartesi Coprocessors and Eigenlayer. With EIP-4788 a trusted beacon block root can be obtained in the Ethereum execution layer. This is used as the input to the coprocessor along with another hash used to bootstrap preimage data retrieval. The beacon block and beacon state are loaded into the coprocessor via the Cartesi preimage oracle and verified against the trusted block root. The validator data in the beacon state is used to derive the relevant values for the Lido oracle which forms the coprocessor output. The output is received back on-chain via a staked operator and can then be consumed by the Lido protocol via the LIP-23 second opinion oracle interface.

This project solves a real and current problem faced by the largest DeFi protocol in the world. After this hackathon I hope it will be considered as a candidate for a further Lido grant.

## Running the Demo

### Local Devnet

Ensure the `.env` file has the 

Begin by building the coprocessor program, car-izing it, and uploading it to the devnet IPFS node so it is available to the operator

```shell
just build
just carize
just upload-to-solver
```

Deploy the contracts with the correct machine hash derived from above

```shell
just deploy-contracts
```

Use the orchestrator CLI to trigger an oracle report for the given beacon slot (e.g. 100)

```shell
just trigger-oracle 100
```

## Challenges Faced


