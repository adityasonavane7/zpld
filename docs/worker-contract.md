# Worker Contract

This document is the specification for implementing a zpld worker. The
`workers/udp-counter` directory is the reference implementation — read the
source alongside this document.

## What a Worker Is

A worker is an independent binary that manages one logical network session.
One IPsec tunnel = one worker. One BGP peer = one worker.

The worker binary must implement the `Worker` trait from `zpld-framework`.
The supervisor treats all workers identically — it does not know or care what
protocol the worker implements.

## The Worker Trait

```rust
pub trait Worker {
    /// Called once at startup. The worker should initialise its internal
    /// state, bind to any sockets, and prepare to handle traffic.
    /// Returns an error if initialisation fails.
    fn init(&mut self, ctx: WorkerContext) -> Result<(), WorkerError>;

    /// Called periodically by the framework. The worker writes a heartbeat
    /// to its StateStore slot. Must complete within the heartbeat deadline.
    fn heartbeat(&mut self) -> Result<(), WorkerError>;

    /// Returns the current stability of the worker. The supervisor will not
    /// initiate a patch while any blocking condition is Unstable.
    fn stability(&self) -> Vec<StabilityCondition>;

    /// Called when the supervisor has decided to drain this worker prior to
    /// replacement. The worker should stop initiating new protocol operations
    /// and write its final state to the StateStore.
    fn drain(&mut self) -> Result<(), WorkerError>;

    /// Called after drain. The worker transfers open file descriptors to the
    /// new worker via the provided FdStore and then exits cleanly.
    fn handoff(&mut self, store: &mut FdStore) -> Result<(), WorkerError>;

    /// Returns a flat key-value map for display by `zpldctl status`.
    /// Values must be human-readable strings. The framework does not parse them.
    fn status_blob(&self) -> HashMap<String, String>;
}
```

## Stability Conditions

```rust
pub struct StabilityCondition {
    pub name: String,
    pub blocking: bool,   // if true, blocks the patch until Stable
    pub status: StabilityStatus,
    pub reason: Option<String>,
}

pub enum StabilityStatus {
    Stable,
    Unstable,
}
```

A worker returns a `Vec<StabilityCondition>` from `stability()`. The
supervisor evaluates all conditions. If any `blocking` condition is
`Unstable`, the patch is held. Non-blocking conditions are advisory —
they appear in `zpldctl status` but do not hold the patch.

Example conditions for an IPsec worker:

```
ike_exchange      blocking  — IKEv2 not mid-exchange
rekey_window      blocking  — SA not within rekey window
dpd_liveness      blocking  — DPD response received recently
sa_lifetime       advisory  — SA lifetime remaining
```

The `udp-counter` worker has no blocking conditions — it is always stable.

## Lifecycle Sequence

```
Supervisor spawns binary
    │
    ▼
Worker::init()        ← bind socket, read config, read StateStore
    │
    ▼
Worker signals Ready  ← framework sends ready signal to supervisor
    │
    ▼
Worker::heartbeat()   ← called every heartbeat_interval (configured in supervisor.toml)
    │
    ...normal operation...
    │
    ▼ (patch initiated)
Worker::drain()       ← stop new ops, write final state to StateStore
    │
    ▼
Worker::handoff()     ← pass FDs to successor via FdStore
    │
    ▼
Worker exits (exit code 0)
```

## StateStore Usage

Each worker has one slot in the StateStore identified by its worker ID.
The framework provides read and write access to this slot.

```rust
// Write state (call from heartbeat and before drain)
ctx.state_store.write(WorkerState {
    version: 1,
    // ... your fields
})?;

// Read state from previous instance (call from init)
if let Some(state) = ctx.state_store.read::<WorkerState>()? {
    self.packet_count = state.packet_count;
}
```

State structs must derive `serde::Serialize` and `serde::Deserialize`.
Always include a `version: u32` field and handle the case where the version
differs from the current binary's version (migration).

## File Descriptor Passing

The old worker passes open file descriptors to the new worker during `handoff`.
The new worker receives them in `init` via `ctx.fd_store`.

```rust
// Old worker — handoff()
store.send("udp_socket", self.socket.as_raw_fd())?;

// New worker — init()
if let Some(fd) = ctx.fd_store.recv("udp_socket")? {
    self.socket = UdpSocket::from_raw_fd(fd);
}
```

If no predecessor exists (first startup), `ctx.fd_store.recv()` returns `None`
and the worker creates its own socket normally.

## Configuration

Each worker reads its own config file. The path is passed by the supervisor
as a command-line argument: `--config /path/to/worker.toml`. Workers choose
their own config format; TOML is conventional.

## Writing a New Worker — Checklist

- [ ] Create a new directory under `workers/`
- [ ] Add a `Cargo.toml` with `zpld-framework` as a dependency
- [ ] Implement the `Worker` trait
- [ ] Define a `WorkerState` struct with a `version` field
- [ ] Read predecessor state in `init`, write state in `heartbeat` and `drain`
- [ ] Pass open sockets via `FdStore` in `handoff`, receive them in `init`
- [ ] Declare stability conditions that reflect your protocol's state
- [ ] Add the crate to the workspace `Cargo.toml`
- [ ] Write a `README.md` and `config/example.toml`
- [ ] Add a Docker test setup under `docker/`
