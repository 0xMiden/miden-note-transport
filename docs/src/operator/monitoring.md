# Monitoring & telemetry

We provide logging to `stdout` and an optional [OpenTelemetry](https://opentelemetry.io/) exporter for our metrics and traces.

OpenTelemetry exporting can be enabled by specifying `--enable-otel` via the command-line or the
`MIDEN_TLNODE_ENABLE_OTEL` environment variable when operating the node.

## Metrics

Various metrics associated with the RPC requests and database operations are provided:

### RPC metrics

| name                               | type                | description                                       |
|------------------------------------|---------------------|---------------------------------------------------|
| `send_note_count`                  | Counter             | number of `send_note` requests                    |
| `send_note_duration`               | Histogram (seconds) | duration of `send_note` requests                  |
| `send_note_size`                   | Histogram (bytes)   | size of received notes in `send_note` requests    |
| `fetch_notes_count`                | Counter             | number of `fetch_notes` requests                  |
| `fetch_notes_duration`             | Histogram (seconds) | duration of `fetch_notes` requests                |
| `fetch_notes_replied_notes_number` | Counter             | number of replied notes in `fetch_notes` requests |
| `fetch_notes_replied_notes_size`   | Histogram (bytes)   | size of replied notes in `fetch_notes` requests   |

### Database metrics

| name                                 | type                | description                                |
|--------------------------------------|---------------------|--------------------------------------------|
| `store_note_count`                   | Counter             | number of `store_note` operations          |
| `store_note_duration`                | Histogram (seconds) | duration of `store_note` operations        |
| `fetch_notes_count`                  | Counter             | number of `fetch_notes` operations         |
| `fetch_notes_duration`               | Histogram (seconds) | duration of `fetch_notes` operations       |
| `maintenance_cleanup_notes_count`    | Counter             | number of `cleanup_old_notes` operations   |
| `maintenance_cleanup_notes_duration` | Histogram (seconds) | duration of `cleanup_old_notes` operations |
}

## Traces

We assign a unique trace (aka root span) to each RPC request.

<div class="warning">

Span and attribute naming is unstable and should not be relied upon. This also means changes here will not be considered
breaking, however we will do our best to document them.

</div>

### RPC traces

<details>
  <summary>Span tree</summary>

```sh
grpc.send_note.request
┕━ db.store_note

grpc.fetch_notes.request
┕━ db.fetch_notes
```

</details>


## Verbosity

We log important spans and events at `info` level or higher, which is also the default log level.

Changing this level should rarely be required - let us know if you're missing information that should be at `info`.

The available log levels are `trace`, `debug`, `info` (default), `warn`, `error` which can be configured using the
`RUST_LOG` environment variable e.g.

```sh
export RUST_LOG=debug
```

## Configuration

The OpenTelemetry trace exporter is enabled by adding the `--enable-otel` flag to the node's start command:

```sh
miden-private-transport-node-bin --enable-otel
```

The exporter can be configured using environment variables as specified in the official
[documents](https://opentelemetry.io/docs/specs/otel/protocol/exporter/).

<div class="warning">
Not all options are fully supported. We are limited to what the Rust OpenTelemetry implementation supports. If you have any problems please open an issue and we'll do our best to resolve it.

</div>
