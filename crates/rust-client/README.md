# Miden Notes Private Transport Client (Rust)

Client library to interact with the Miden Transport Layer.

## Crate Features

| Features     | Description                                                                                                                                               |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `idxdb`      | Includes `IndexedDatabase`, an IndexedDB implementation of the `DatabaseBackend` trait. **Disabled by default.**                                                          |
| `sqlite`     | Includes `SqliteDatabase`, a SQLite implementation of the `DatabaseBackend` trait. This relies on the standard library. **Disabled by default.**                                                           |
| `tonic`      | Includes employs a `std`-compatible Tonic client to communicate with Miden Transport Layer node. This relies on the `tonic` for the inner transport.  **Disabled by default.**                                                        |
| `web-tonic`  | Includes a `wasm`-compatible Tonic client to communicate with the Miden Transport Layer node. This relies on `tonic-web-wasm-client` for the inner transport. **Disabled by default.**                                   |
| `testing`    | Enables functions meant to be used in testing environments. **Disabled by default.**             |

Features `sqlite` and `idxdb` are mutually exclusive, the same goes for `tonic` and `web-tonic`.
Both `tonic` and `web-tonic` employ the same `GrpcClient`, however using different inner services.

### `DatabaseBackend` and `TransportLayer` implementations

The library user can provide their own implementations of `DatabaseBackend` and `TransportClient` traits, which can be used as components of `TransportLayerClient`, though it is not necessary. The `DatabaseBackend` trait is used to persist the state of the client, while the `TransportLayer` trait is used to communicate via [gRPC](https://grpc.io/) with the Miden node.

The `sqlite` and `tonic` features provide implementations for these traits using [sqlx](https://github.com/launchbadge/sqlx) and [Tonic](https://github.com/hyperium/tonic) respectively. The `idxdb` and `web-tonic` features provide implementations based on [IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API) and [tonic-web](https://github.com/hyperium/tonic/tree/master/tonic-web) which can be used in the browser.


## License
This project is [MIT licensed](../../LICENSE).
