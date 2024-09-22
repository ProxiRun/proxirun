# Chain Listener for Aptos Transaction Stream

The Chain Listener is a Rust library designed to connect to the Aptos Transaction Stream Service. It intercepts gRPC requests with an authorization token, filters for events related to the ProxiRun smart contract, and parses these events into structured types defined in the ProxiRun SDK. The parsed events are then pushed into a channel for downstream processing.

Since it connects to the [Aptos Transaction Stream Service](https://aptos.dev/en/build/indexer/txn-stream), usage requires a auth token.

## Features

- **gRPC Connection**: Utilizes gRPC to connect to the Aptos Transaction Stream Service, ensuring efficient and real-time transaction updates.
- **Authorization Interceptor**: Implements a custom interceptor to handle authentication with the service using a Bearer token.
- **Event Filtering**: Listens for transaction events specifically related to the ProxiRun contract, filtering them based on the contract’s module ID.
- **Parallel Processing**: Uses the `rayon` library for parallel processing of incoming transaction events, improving performance and responsiveness.
- **Channel Integration**: Sends parsed events through an `UnboundedSender` channel, allowing for easy integration with other components of your application.


### Example Integration and Usage

See worker for example of integration and usage 

