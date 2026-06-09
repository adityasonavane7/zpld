# Architecture

## Core Insight

The kernel's networking stack persists independently of userspace processes.
IPsec XFRM Security Associations, routing table entries, and socket buffers
all survive a process restart. zpld exploits this: workers are thin controllers
over kernel state, not the state itself. Replacing a worker replaces the
controller, not the data.

## Process Model

```
zpldctl ──(Unix socket)──▶ zpld-supervisor ──(Unix socket)──▶ zpld-worker-A
                                  │                           zpld-worker-B
                                  │                           ...
                            StateStore (mmap'd file)
                                  │
                            Kernel (XFRM, FIB, sockets)
```

### Supervisor

One instance per host. Responsibilities:

- Reads `supervisor.toml` and spawns the configured workers
- Monitors worker health via periodic heartbeats
- Manages the lazy update lifecycle for both workers and itself
- Exposes a Unix socket at `/run/zpld/supervisor.sock` for `zpldctl`
- Writes and reads the StateStore to coordinate with workers

The supervisor is itself live-patchable. It patches via `fork()` + `exec()`:
the parent waits for the child to signal readiness before exiting.

### Workers

One process per logical session. A worker:

- Implements the `Worker` trait from `zpld-framework`
- Declares stability conditions specific to its protocol
- Writes its state to its StateStore slot before draining
- Accepts open file descriptors from its predecessor via SCM_RIGHTS
- Signals readiness to the supervisor when it is fully operational

Workers are generic. The supervisor does not know what protocol a worker
implements — it only speaks the framework's IPC protocol.

### StateStore

A memory-mapped file at `/run/zpld/state.db`. Layout:

```
[ Header: magic, version, global lock ]
[ SupervisorRecord: pid, socket path, version ]
[ WorkerRecord[0]: id, pid, binary path, socket path, state, protocol state ]
[ WorkerRecord[1]: ... ]
...
```

Serialized with `bincode`. Each record carries a version field. When a new
binary reads a record written by an older binary, it runs the appropriate
migration. See [state-store.md](state-store.md) for the full format.

## Lazy Update Lifecycle

```
zpldctl update worker <id>
  │
  ▼
Supervisor marks worker: UpdatePending
  │
  ▼
Supervisor polls StabilityConditions (worker-reported)
  │  (waits until all blocking conditions report Stable)
  ▼
Supervisor spawns new worker binary in shadow mode
  │
  ▼
New worker: reads StateStore, initializes, signals Ready
  │
  ▼
Supervisor sends Drain to old worker
Old worker: stops new operations, writes final state to StateStore
  │
  ▼
Supervisor transfers FDs (SCM_RIGHTS): old worker → new worker
  │
  ▼
Old worker exits. New worker promotes to active.
Supervisor updates WorkerRecord (new pid, version, socket path).
```

## Failure Modes

| Failure | Behaviour |
|---|---|
| New worker fails to start | Mark update failed, keep old worker, emit event |
| New worker fails readiness check | Kill new worker, keep old, backoff and retry |
| Old worker dies during handoff | New worker fast-promotes; takes over kernel state |
| Worker dies unexpectedly | Supervisor detects via heartbeat loss; auto-restarts if configured |
| Supervisor crashes | Workers continue running; kernel state unaffected. On restart, supervisor reads StateStore and reconnects |
| Supervisor patch fails after fork | Parent (old supervisor) is still running; parent aborts the patch |

## Security Boundaries

The supervisor control socket (`/run/zpld/supervisor.sock`) must be
readable only by root or a dedicated `zpld` group. Anyone with access to
this socket can trigger worker patches and read worker state.

Workers run as root (required for `CAP_NET_ADMIN` to manage XFRM state).
The supervisor drops privileges not needed for orchestration after startup.
