# ProxiRun Backend

## Overview

ProxiRun is a decentralized marketplace for compute built on the Aptos blockchain. It provides a platform to request computations in a distributed way, utilizing a fair and transparent smart contract-based marketplace. Currently, ProxiRun focuses on ML/AI generations (text completion, image generation, voice generation) with plans to expand support to ZK proofs and access to a wider selection of ML models.

This repository contains the backend components for the ProxiRun project, organized as a Rust workspace with multiple members.

## Repository Structure

The workspace consists of the following main components:

1. Chain Listener
2. Orchestrator Service
3. ProxiRun SDK
4. Worker

Each component plays a crucial role in the ProxiRun ecosystem:

### Chain Listener

The Chain Listener is a Rust library that connects to the Aptos Transaction Stream Service. It intercepts gRPC requests to inject the auth token, filters for events related to the ProxiRun smart contract, and parses these events into structured types defined in the ProxiRun SDK.

Key features:
- gRPC connection to Aptos Transaction Stream Service
- Custom authorization interceptor
- Event filtering for ProxiRun contract events
- Parallel processing of incoming events
- Channel integration for downstream processing

### Orchestrator Service

The Orchestrator Service manages work requests, auction finalization, and result submissions for the decentralized compute marketplace on the Aptos blockchain.

Key features:
- Listens for blockchain events related to new work requests
- Schedules and executes auction finalization
- Handles task payload and definition retrieval
- Manages submission of text and image results
- Interacts with the ProxiRun smart contract for various operations

### ProxiRun SDK

The ProxiRun SDK is a Rust library that facilitates interaction with the ProxiRun decentralized marketplace for compute tasks on the Aptos blockchain.

Key components:
- Constants (contract address, module name, service URLs)
- Method wrappers for contract interactions
- Event definitions mirroring smart contract events
- Type definitions for service interactions and contract operations

### Worker

The ProxiRun Worker is a sample application designed to listen for events from the ProxiRun contract, bid on auctions, and process tasks upon winning an auction.

Key features:
- Event listening for new work requests and auction wins
- Automatic auction bidding based on event data
- Task processing and result submission

## Getting Started

### Prerequisites

- Rust and Cargo installed
- Access to Aptos testnet
- Required environment variables (see individual component READMEs for details)

### Setup

1. Clone the repository:
   ```
   git clone https://github.com/your-repo/proxirun-backend.git
   cd proxirun-backend
   ```

2. Set up environment variables:
   Create a `.env` file in the root directory and add the necessary variables (refer to individual component READMEs for specific requirements).

3. Build the project:
   ```
   cargo build
   ```

### Running Components

Refer to the individual README files in each component's directory for specific instructions on running and using each part of the system.

## Usage

For detailed usage instructions and API endpoints, please refer to the README files in each component's directory:

- [Chain Listener README](./chain_listener/README.md)
- [Orchestrator Service README](./orchestrator/README.md)
- [ProxiRun SDK README](./proxirun_sdk/README.md)
- [Worker README](./worker/README.md)


## Contact

For more information about ProxiRun and its ecosystem, please visit our [GitHub organization profile](https://github.com/ProxiRun).