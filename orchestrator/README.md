# ProxiRun Orchestrator Service

## Overview

The ProxiRun Orchestrator Service is a crucial component of the ProxiRun ecosystem, managing work requests, auction finalization, and result submissions for the decentralized compute marketplace on the Aptos blockchain.

## Features

- Listens for blockchain events related to new work requests
- Schedules and executes auction finalization
- Handles task payload and definition retrieval
- Manages submission of text and image results
- Interacts with the ProxiRun smart contract for various operations

## Prerequisites

- Rust and Cargo
- Access to Aptos testnet
- ProxiRun SDK
- Environment variables:
  - `INDEXER_AUTH_KEY`
  - `ADMIN_PRIVATE_KEY`
  - `ORCHESTRATOR_URL`
  - `ORCHESTRATOR_PORT`

## Setup

1. Clone the repository
2. Set up the required environment variables (use a `.env` file or system environment)
3. Ensure the `uploads` directory exists in the project root

## Running the Service

```sh
cargo run
```

## API Endpoints

- GET `/request-details/{id}`: Retrieve task definition
- GET `/request-payload/{id}`: Retrieve task payload
- POST `/submit-text/{id}`: Submit text result
- POST `/submit-image/{id}`: Submit image result

## Configuration

- `INDEXER_URL`: Set to Aptos testnet indexer
- `TESTNET_NODE`: Set to Aptos testnet fullnode
- `DELTA_TIME`: Auction finalization delay (in microseconds)

## Dependencies

- actix-web: Web framework
- aptos-sdk: Aptos blockchain interaction
- proxirun-sdk: ProxiRun specific functionalities
- tokio: Asynchronous runtime

For more detailed information about ProxiRun and its ecosystem, please visit our [GitHub organization profile](https://github.com/ProxiRun).