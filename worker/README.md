# Worker for ProxiRun Auctions

The ProxiRun Worker is a sample application designed to listen for events from the ProxiRun contract, bid on auctions, and process tasks upon winning an auction. This worker integrates with the Aptos blockchain and handles real-time events to facilitate seamless interactions with the ProxiRun ecosystem. 

## Features

- **Event Listening**: Listens for contract events related to new work requests and auction wins.
- **Auction Bidding**: Automatically places bids on auctions based on incoming event data.
- **Task Processing**: Upon winning an auction, retrieves task details and processes the work before submitting results to the orchestrator.

## Getting Started

### Prerequisites

- Set up your environment with the required API keys and orchestrator URL in a `.env` file:
  ```
  INDEXER_AUTH_KEY=your_auth_key
  ORCHESTRATOR_URL=your_orchestrator_url
  ORCHESTRATOR_PORT=your_orchestrator_port
  ```

### Running the Worker

1. **Clone the Repository**: Ensure you have the workspace set up.
2. **Install Dependencies**: Use `cargo build` to install necessary dependencies.
3. **Run the Worker**: Execute the following command in your terminal:
   ```bash
   cargo run --bin worker
   ```

### Workflow

- The worker listens for `OnNewWorkRequest` events.
- Upon receiving a new request, it decides whether or not to bid on a request 
- When an auction is won, it processes the task associated with the request.

