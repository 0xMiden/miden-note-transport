// Load the WASM module using the workaround pattern
import loadWasm from "../dist/wasm.js";
const wasm = await loadWasm();

const {
  TransportLayerWebClient: WasmTransportLayerWebClient,
  Note,
  NoteHeader,
  NoteTag,
  NoteId,
  Address,
  mockAddress,
  mockNoteP2IDWithAddresses,
  parse_bech32_address,
  create_note_tag_from_int,
  parse_hex_note,
  get_note_info,
  get_note_tag,
  get_note_sender_bech32,
} = wasm;

/**
 * TransportLayerWebClient is a wrapper around the underlying WASM TransportLayerWebClient object.
 */
export class TransportLayerWebClient {
  constructor() {
    this.wasmClient = null;
  }

  /**
   * Factory method to create and initialize a TransportLayerWebClient instance.
   *
   * @param {string} url - The transport server URL
   * @returns {Promise<TransportLayerWebClient>} The fully initialized client
   */
  static async create(url) {
    const instance = new TransportLayerWebClient();
    try {
      await instance.connect(url);
      return instance;
    } catch (error) {
      throw new Error(`Failed to create client: ${error.message || error}`);
    }
  }

  /**
   * Initialize the client with a transport server URL
   *
   * @param {string} url - The transport server URL
   * @returns {Promise<void>}
   */
  async connect(url) {
    this.wasmClient = new WasmTransportLayerWebClient();
    await this.wasmClient.connect(url);
  }

  /**
   * Send a note to the transport layer
   *
   * @param {Note} note - The note to send
   * @param {Address} address - The address to send to
   * @returns {Promise<NoteId>} The ID of the sent note
   */
  async sendNote(note, address) {
    if (!this.wasmClient) {
      throw new Error("Client not initialized. Call connect() first.");
    }
    return await this.wasmClient.sendNote(note, address);
  }

  /**
   * Fetch notes from the transport layer
   *
   * @param {Array<NoteTag>} tags - Array of note tags to filter by (can be single tag in array)
   * @returns {Promise<Array<Note>>} Array of note objects
   */
  async fetchNotes(tags) {
    if (!this.wasmClient) {
      throw new Error("Client not initialized. Call connect() first.");
    }
    return await this.wasmClient.fetchNotes(tags);
  }

  /**
   * Create a mock address for testing purposes (from rust-client)
   *
   * @returns {Address} A mock address
   */
  static mockAddress() {
    return mockAddress();
  }

  /**
   * Create a mock P2ID note with specified sender and target addresses (from rust-client)
   *
   * @param {Address} sender - The sender address
   * @param {Address} target - The target address
   * @returns {Note} A mock note
   */
  static mockNoteP2IDWithAddresses(sender, target) {
    return mockNoteP2IDWithAddresses(sender, target);
  }
}

// Re-export WASM types for direct use
export {
  Note,
  NoteHeader,
  NoteTag,
  NoteId,
  Address,
  mockAddress,
  mockNoteP2IDWithAddresses,
  parse_bech32_address,
  create_note_tag_from_int,
  parse_hex_note,
  get_note_info,
  get_note_tag,
  get_note_sender_bech32,
};

// AddressInterface is a type, not an enum, so we define it as a constant
export const AddressInterface = {
  Unspecified: "Unspecified",
  BasicWallet: "BasicWallet"
};
