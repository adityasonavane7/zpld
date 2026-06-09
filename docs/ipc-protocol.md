# IPC Protocol

zpld has two IPC channels:

1. **Control protocol** — `zpldctl` ↔ Supervisor. Used by the CLI to issue
   commands and read status.
2. **Worker protocol** — Supervisor ↔ Worker. Used for heartbeats, lifecycle
   commands, and handoff coordination.

Both channels use Unix domain sockets with length-prefixed messages serialized
using `bincode`.

## Message Framing

All messages are framed the same way:

```
[ u32: message length in bytes ][ message bytes ]
```

The length field is big-endian. The message bytes are a `bincode`-serialized
enum. There is no session handshake — the socket is opened, messages are
exchanged, and the socket is closed.

## Control Protocol (zpldctl ↔ Supervisor)

Socket path: `/run/zpld/supervisor.sock`

### Request

```rust
enum ControlRequest {
    List,
    Status { worker_id: String },
    UpdateWorker { worker_id: String },
    UpdateSupervisor,
    CancelUpdate { worker_id: String },
    AddWorker { config_path: String },
    RemoveWorker { worker_id: String },
    StreamEvents,
}
```

### Response

```rust
enum ControlResponse {
    WorkerList(Vec<WorkerSummary>),
    WorkerStatus(WorkerDetail),
    Ok,
    Error(String),
    Event(WorkerEvent),   // used for StreamEvents (multiple responses)
}

struct WorkerSummary {
    id: String,
    binary_path: String,
    version: String,
    state: WorkerState,
    health: WorkerHealth,
}

struct WorkerDetail {
    summary: WorkerSummary,
    stability: Vec<StabilityCondition>,
    status_blob: HashMap<String, String>,  // worker-specific fields
}
```

## Worker Protocol (Supervisor ↔ Worker)

Each worker creates a Unix socket at a path it registers in the StateStore.
The supervisor connects to this socket.

### Supervisor → Worker

```rust
enum SupervisorCommand {
    Heartbeat,
    StabilityCheck,
    Drain,
    Handoff { fd_socket_path: String },
    Shutdown,
}
```

### Worker → Supervisor

```rust
enum WorkerResponse {
    HeartbeatAck,
    StabilityReport(Vec<StabilityCondition>),
    DrainAck,
    HandoffComplete,
    StatusBlob(HashMap<String, String>),
    Error(String),
}
```

## Handoff Coordination

File descriptor passing (SCM_RIGHTS) happens over a separate, temporary Unix
socket created specifically for the handoff. The path is communicated via the
`Handoff` command.

Sequence:

```
Supervisor → Old Worker: Drain
Old Worker → Supervisor: DrainAck
Supervisor creates temp socket at /run/zpld/handoff-<id>.sock
Supervisor → New Worker: connect to handoff socket and receive FDs
Supervisor → Old Worker: Handoff { fd_socket_path }
Old Worker sends FDs over temp socket (SCM_RIGHTS)
New Worker receives FDs
Old Worker → Supervisor: HandoffComplete
New Worker signals Ready
Supervisor removes temp socket
Old Worker exits
```
