# Miden Private Transport Layer - Web Client

## Building and Testing the Web Client

> [!NOTE]
> This crate follows the structure of the Miden [web-client](https://github.com/0xMiden/miden-client/tree/next/crates/web-client).


If you're interested in contributing to the web client and need to build it locally, you can do so via:

```
yarn install
yarn build
```

This will:
* Install all JavaScript dependencies,
* Compile the Rust code to WebAssembly,
* Generate the JavaScript bindings via wasm-bindgen,
* And bundle the SDK into the dist/ directory using Rollup.

To run integration tests after building, use: 
```
yarn test
```

This runs a suite of integration tests to verify the SDK’s functionality in a web context.

A simple app to send and fetch notes is provided. Use:

```
yarn app
```

And navigate to `localhost:3000` to use the transport layer through a browser interface.

## Usage

The Miden Private Transport Web Client provides a TypeScript/JavaScript interface for interacting with the Miden Private Transport Layer. This library enables you to send and fetch notes through a web browser using WebAssembly.

### Installation

Since this library is not yet published to npm, you'll need to build it locally:

```bash
# Clone the repository
git clone <repository-url>
cd miden-private-transport/crates/web-client

# Install dependencies and build
yarn install
yarn build
```

The built library will be available in the `dist/` directory.

### Basic Setup

```typescript
import { 
  TransportLayerWebClient, 
  Note, 
  Address, 
  NoteTag,
  parse_bech32_address,
  create_note_tag_from_int,
  parse_hex_note
} from './dist/index.js';

// Create a new client instance
const client = new TransportLayerWebClient();

// Connect to the transport server
await client.connect('http://localhost:8080');
```

### Sending Notes

```typescript
// Parse a bech32 address
const targetAddress = parse_bech32_address('miden1q...');

// Parse a note from hex string
const note = parse_hex_note('0x1234...');

// Send the note
const noteId = await client.sendNote(note, targetAddress);
console.log('Note sent with ID:', noteId);
```

### Fetching Notes

```typescript
// Create a note tag from an integer
const noteTag = create_note_tag_from_int(123);

// Fetch notes with the specified tag
const notes = await client.fetchNotes(noteTag);
console.log(`Found ${notes.length} notes`);

// Process each note
for (const note of notes) {
  console.log('Note details:', note);
}
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
