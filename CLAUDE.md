# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

A Rust game server framework (see `README.md`). The workspace is split into a reusable server library (`orion`), a proc-macro helper crate (`orion-macros`), a shared protobuf crate that code-generates at build time (`protobuf`), and one concrete server binary (`gate`). Additional server types (e.g. `logic`, `battle` — referenced in `config/proto.json`) are expected to be added as new workspace members that depend on `orion` and `protobuf`.

## Common commands

All commands are run from the repository root; `gate` reads `config/*.json` using relative paths, so tests and the binary must start from the workspace root.

- Build everything: `cargo build`
- Run the gate binary: `cargo run -p gate` (requires NATS on `NATS_URL` / default `nats://localhost:4222` and Redis on `REDIS_URL` / default `redis://localhost:6379`)
- Run all tests: `cargo test`
- Run tests for a single crate: `cargo test -p gate` (or `-p orion`, `-p orion-macros`, `-p protobuf`)
- Run a single test by path: `cargo test -p gate client::socket_client::tests::test_receive_handshake`

### Runtime environment variables (read by `gate`)

- `server_id` — `u32`, must be > 0. Used as this process's unique id in NATS subjects.
- `server_type` — string tag (gate sets this to `"gate"` itself in `main.rs`; other servers must set their own).
- `NATS_URL`, `REDIS_URL`, `ADDR`, `PORT` — connection/bind settings with the defaults above / `127.0.0.1:9001`.

## Architecture

### Workspace layout

- `orion/` — core framework. Public surface is re-exported from `orion/src/lib.rs`: `Application`, `appinfo`, TCP server (`serve_tcp`, `SocketListener`, `SocketHandle`, `TcpSocketHandle`), NATS (`nats_client`, `nats_msg`, `rpc_subscriber`), Redis helper (`async_redis`), and the `rpc` module + `register_rpc!` macro.
- `gate/` — binary crate. Client-facing TCP gateway that terminates the wire protocol, authenticates clients, and forwards messages onto NATS as RPC requests / notifications to backend servers. It also listens for server→client messages on NATS and writes them back to the right socket.
- `orion-macros/` — proc-macro crate. Currently exposes `#[init_tracing]`, which is applied to `async fn main` (see `gate/src/main.rs`) and injects `tracing_subscriber` setup at the top of the function body.
- `protobuf/` — shared generated code. `build.rs` walks `protobuf/src/**/*.proto`, compiles them with `prost-build`, and then overwrites `protobuf/src/lib.rs` to `include!` every generated file. `protobuf/src/lib.rs` is therefore a build artifact — don't hand-edit it; edit `build.rs` or add `.proto` files. Game protocol messages live under `src/game-proto/`; RPC service definitions live under `src/rpc/<servertype>/` and are exposed as `protobuf::rpc::*`.

### Process lifecycle

Every server binary follows the pattern in `gate/src/main.rs`:

1. `#[orion_macros::init_tracing]` + `#[tokio::main]` on `main`.
2. `orion::setup_panic_hook()` — appends panic info to `panic.log`.
3. Set `server_type` env var; connect NATS and Redis; stash them in a per-crate `global` module using `OnceLock` (see `gate/src/global.rs`).
4. Start transport(s) and NATS subscribers as detached `tokio::spawn` tasks.
5. `register_rpc!` to wire handlers into the global `RpcRouter`.
6. `orion::Application::new().start().await` blocks on SIGINT/SIGTERM and then runs `shutdown`.

`appinfo()` in `orion/src/app.rs` is a process-wide `OnceLock<AppInfo>` that reads `server_id` and `server_type` from env on first access. It panics if `server_id` is `0`, and many NATS subject strings depend on it — always set `server_id` before any code path that calls `appinfo()`.

### Client wire protocol (TCP, in `gate`)

Two framing layers stacked together; `serve_tcp` in `orion/src/net/tcp.rs` handles the outer packet layer, and `gate/src/protocol/` handles the inner message layer.

- **Packet** (`gate/src/protocol/packet.rs`): `type (1B) | length (3B, big-endian) | body`. Types: `Handshake`, `HandshakeAck`, `Heartbeat`, `Data`, `Kick`, `Error`.
- **Message** (inside `Data` packets, `gate/src/protocol/message.rs`): `msg_type (1B) | [reqid (1B) if Request/Response] | [proto_id (2B) if Request/Notify/Push] | payload`.

Client state machine (`gate/src/client/socket_client.rs`): `WAIT_FOR_HANDSHAKE → WAIT_FOR_HANDSHAKE_ACK → READY`. Handshake payload carries `uid_len | uid | client_version`. After READY, the client sends heartbeats every `HEARTBEAT_INTERVAL` (30s); a heartbeat watchdog in `Client::new` closes the socket if none arrives within `2 × HEARTBEAT_INTERVAL`.

`ClientManager<T: NetClient>` (`gate/src/client/mod.rs`) keeps three maps — socket_id → client, uid → socket_id, socket_id → uid — behind `Mutex`es so TCP accept loop, NATS subscriber, and RPC callers can all look up clients by either key.

### Gate → backend server routing (NATS)

On `Data` packets, `gate/src/client/socket_client.rs` looks up the protocol string in `config/proto.json` (format: `"<server_type>-<MessageName>"`), checks `config/servers.json` for `stateless`, and publishes to `handler.<server_type>` (stateless) or — per the TODO — to a specific instance subject. Payload is `nats_msg::encode(socket_id, proto_id, reqid, uid, gate_server_id, data)` (see `orion/src/nats/nats_msg.rs`).

Request flow:
- `Request` → `publish_with_reply("handler.<type>", "<gate_uuid>.reply.<reqid>", payload)`
- `Notify` → `publish("handler.<type>", payload)`

Backend servers run `orion::rpc_subscriber::queue_subscribe(nats)` (queue group = `server_type`, subject = `rpc.<server_type>.>`) or `subscribe` for per-instance subjects. `find_route` in `orion/src/nats/rpc_subscriber.rs` parses the subject `rpc.<server_type>.<Service.Method>.<...>` by slicing between the 2nd and 4th dots, then dispatches to the route registered with `register_rpc!`.

### Backend → client messages (gate listener)

`gate/src/s2clistener.rs` subscribes to `<gate_uuid>.>`. Subjects of the form `<uuid>.push.<...>` / `<uuid>.reply.<...>` are dispatched to the right client via `ClientManager::get_client(socket_id)` and sent back as `Push` / `Response` messages.

### RPC handler registration

`orion::rpc::RpcRouter` is a global `OnceLock<RpcRouter>`. `register_rpc_routes` seeds it exactly once — subsequent registrations are silently ignored. The `register_rpc!` macro takes `"Service.Method" => async_fn` pairs; the handler signature must be `async fn(Req) -> Resp` where both `Req` and `Resp` implement `prost::Message + Default`. See `gate/src/rpc_helper.rs` for the canonical example.

### Config files

`config/servers.json` — per-server-type flags (currently only `stateless`). `config/proto.json` — ordered list of `"<server_type>-<MessageName>"` strings; the index is the wire `proto_id`, so **never reorder or remove entries** without coordinating with clients; only append. Both files are loaded lazily via `OnceLock` in `gate/src/config.rs` using paths relative to the current working directory.

## Conventions

- The public API of `orion` is the re-exports in `orion/src/lib.rs`. Add new public items there rather than exposing internal module paths.
- When adding a new `.proto` file, put it under `protobuf/src/` (game messages under `src/game-proto/`, RPC services under `src/rpc/<servertype>/`). The `build.rs` picks it up automatically, but `protobuf/src/lib.rs` currently only re-exports `game.rs` and `rpc.gate.rs`; extend `lib.rs` (or `build.rs`'s generated content) when introducing a new package.
- Globals (NATS client, Redis connection manager, `AppInfo`, `RpcRouter`) all use `OnceLock`. Set them during `main` before spawning tasks that read them; reads after init are lock-free clones of `Arc`-like handles.
- Tests that touch `gate`'s config or bind real sockets assume they run from the workspace root and that ports (`8080`, `9001`, …) are free.
