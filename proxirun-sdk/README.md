# ProxiRun SDK

## Overview

The ProxiRun SDK is a Rust library that facilitates interaction with the ProxiRun decentralized marketplace for compute tasks on the Aptos blockchain. This SDK provides essential components for integrating with the ProxiRun ecosystem.

## Components

1. **Constants**
   - Contract address
   - Module name
   - Orchestrator service URL and port

2. **Method Wrappers**
   - `bid`: Submit a bid for an ongoing auction
   - `finalize_auction`: Determine the auction winner for a compute request
   - `commit`: Called by admin to confirm worker submission of generated output

3. **Event Definitions**
   - Mirrors events emitted by the ProxiRun smart contract

4. **Type Definitions**
   - Structures for interacting with the orchestrator service
   - Types for handling request data and generated output submission
   - Definitions for smart contract interactions (auction finalization and work commitment)

## Usage

See worker and chain_listener for example usage 
