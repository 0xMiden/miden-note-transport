# Miden Notes Private Transport Node

Node/server implementation of the Miden Transport Layer for private notes.

## API
Messages exchanged with the protocol using gRPC.
Please see the `miden-private-transport-proto` crate for the employed Protobuf messages and services.

Clients can interact with the server,
- `send_note()` receives an incoming note and stores it in the database. The note details can be
encrypted;
- `fetch_notes()` process a notes-request by note tag, with pagination based on a timestamp. The
node timestamps notes when it receives them (on `send_note()` requests);
- `stream_notes()` is a subscription mechanism by note tag for real-time note-fetching. Received notes by the
node are sent to subscribed client;
- `stats()` provides simple insights into database statistics.

## Telemetry
Metrics and traces to monitor the node state are provided.
While metrics provide insights into general requests stats, traces can provide insights into specific
requests.
Metrics and traces can be exported following using the [OpenTelemetry](https://opentelemetry.io) framework.

## License
This project is [MIT licensed](../../LICENSE).
