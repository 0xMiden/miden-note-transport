# Miden Private Transport Layer Roadmap

## Current Architecture Overview

The Miden Private Transport Layer is a communications system for secure exchange of private notes with the following current features:
- Basic note sending/fetching and streaming via gRPC
- Node (centralized) with,
	- SQLite database;
	- OpenTelemetry metrics and tracing;
	- Load testing tool;
	- Docker deployment setup.	
- Rust and Web client with WASM support,
	- Client database for user, note data persistence (SQLite and IndexedDB), following the Miden edge-focused paradigm;
	- CLI and web UI applications for testing / showcasing;
	- Ready for end-to-end encryption.

### Roadmap

The following roadmap with estimated durations assume 1 full-time senior engineer.

## 1. Encryption & Key Management (1 week)

Asymmetric encryption scheme based X25519 + AES previously tested,
- Keys are exchanged through some other channel;
- Note details are encrypted, note header plaintext;
- Basic filesystem-based key management/storage.

Reintegration using the Address structure, which contains the encryption key of the receiver.
Node implementation is agnostic of this, remains unchanged.

Note: Keys could potentially be shared through the transport layer. This was also tested.
However, there is still an authentication issue: how to ensure a registered key belongs to some user/account ID? Some proof could potentially be used to, for example, prove that the user registering a key knows the secret data that derives to its account ID.

## 2. Decentralization (6-8 weeks)

Current state based on a centralized client-server architecture.

Moving to a decentralized architecture, we try to leverage,
- Distributed storage;
- Transport Layer fault tolerance;
- Potentially enhanced user privacy.

Move to a decentralized architecture, featuring,
- libp2p-based networking;
- Note discovery;
- Gossip protocol for node propagation.

Currently, to fetch notes a timestamp-based pagination is employed.
However this could be unreliable in decentralized systems given the potentially different clocks. Explore block number -based pagination.
- The block number is a canonical identifier through the whole network, however notes may still reach out-of-order;
- Note fetching will need some negative offset in pagination to account for delays. Some fetched notes will be repeated (the current client implementation is already prepared for these duplicates).

## 3. Enhanced Privacy & Anonymity (? Weeks)

Currently, note tags can be derived from the recipient address (default) or be fully random.
Which tags (random scenario) to be used must be previously agreed by the sender and receiver.

Use of random tags (unlinked to the address) provides a high degree of privacy.

To further boost privacy for the client:
- Leverage network decentralization: use different nodes to issue requests;
- Prefer non-periodic requests: add random delays to requests.

## 4. Group Communication (? weeks)

While currently no encryption is employed, it is expected that through the Address structure, encrypted communications will be one-to-one.

Implement 1-to-n communication, with encryption enabled.
- The Transport Layer is agnostic to the underlying encryption mechanism.
- An efficient key distribution mechanism will likely be required for sharing the encryption keys, e.g,
    - Identity-Based Encryption (IBE) requires a user (potentially the sender) to generate keys for each user;
    - Symmetric encryption, a single key could be used by the whole group.
- The Transport Layer could potentially be employed for key exchanges.

## 5. Fees (? weeks)

Explore a fee-collection system.
Fees must be low, especially considering other (non-Miden) free communication systems.

Potential design:
- Auth-token -based authentication;
- Users issues (Miden) tokens to the node operator in exchange for an auth-token providing access to N requests;
- Different fee levels based on number and types of requests.


