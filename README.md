# zpld — Zero Packet Loss Daemon Framework

A framework for writing live-patchable network daemons where software updates
do not disrupt active sessions or drop traffic.

## The Problem

When a network daemon (VPN, BGP, IKE) is patched or restarted, active sessions
reset. BGP peerings flap. VPN tunnels renegotiate. Traffic drops during the
window between the old process exiting and the new one coming up.

zpld solves this by separating the dataplane from the control plane and
providing a supervised process model where both the supervisor and worker
processes can be replaced without disrupting active sessions.

## How It Works

The kernel's networking stack (routing tables, IPsec XFRM SAs, socket buffers)
persists independently of any userspace process. zpld exploits this: worker
processes are thin controllers over kernel state. When a worker is patched, the
new process inherits kernel state and open file descriptors from its predecessor
before the predecessor exits. From the peer's perspective, nothing happened.

```
                    ┌─────────────────────────────────────┐
  zpldctl ─────────▶│          zpld-supervisor            │
  (CLI tool)        │                                     │
                    │  Registry │ PatchScheduler │ Health │
                    └──────────────────┬──────────────────┘
                                       │ manages
                    ┌──────────────────┼──────────────────┐
                    │                  │                  │
             ┌──────▼──────┐   ┌──────▼──────┐           ...
             │   worker A  │   │   worker B  │
             │  (ipsec)    │   │  (bgp)      │
             └──────┬──────┘   └──────┬──────┘
                    │                 │
                    ▼                 ▼
             ┌─────────────────────────────────┐
             │         Kernel (XFRM, FIB)      │
             │    traffic never leaves here    │
             └─────────────────────────────────┘
```

### Supervisor

A single long-running process that manages worker lifecycles. Reads a config
file defining which worker binaries to spawn and with what configuration. Exposes
a Unix socket for the `zpldctl` CLI. The supervisor itself is live-patchable.

### Workers

One process per logical session (one IPsec tunnel, one BGP peer, etc.). Each
worker is an independent binary that implements the worker contract defined by
`zpld-framework`. Workers declare their own stability conditions — the supervisor
will not initiate a patch while a worker reports instability.

### StateStore

A memory-mapped file at `/run/zpld/state.db`. Contains versioned per-worker
state records. Both the supervisor and workers read and write this file.
Because state is external to any process, either can be replaced and the
replacement immediately has full context.

### Lazy Updates

Patches are applied lazily. When `zpldctl update worker <id>` is issued:

1. Supervisor marks the worker as update-pending
2. Supervisor polls the worker's stability conditions
3. Once the worker reports stable, the supervisor starts the new binary
4. New worker reads state from StateStore, initializes, signals ready
5. Supervisor drains the old worker, transfers open file descriptors via SCM_RIGHTS
6. Old worker exits, new worker takes over
7. From the network peer's perspective, nothing changed

## Repository Layout

```
zpld/
├── crates/
│   ├── zpld-framework/     Core library. Supervisor logic, worker contract,
│   │                       StateStore, IPC protocols. No daemon-specific code.
│   │
│   ├── zpld-supervisor/    Supervisor binary. Reads config, manages workers,
│   │                       exposes control socket for zpldctl.
│   │
│   └── zpldctl/            CLI tool. Talks to the supervisor control socket.
│
└── workers/                Worker implementations. Each worker is an
    │                       independent binary that uses zpld-framework.
    │                       Each worker has its own README and test setup.
    │
    └── udp-counter/        Reference worker. Counts UDP packets from a
                            configured source. The recommended starting point
                            for understanding how to implement a zpld worker.
```

## Writing a Worker

Workers are independent binaries that implement the worker contract from
`zpld-framework`. See [workers/udp-counter](workers/udp-counter/README.md)
for a fully documented reference implementation covering:

- Implementing the `Worker` trait
- Declaring stability conditions
- Writing and reading from the StateStore
- Handling drain, handoff, and exit lifecycle events
- Passing file descriptors to a successor process

## Workers

| Worker | Status | Description |
|---|---|---|
| `udp-counter` | In progress | Reference worker. Counts UDP packets from a source. |
| `ipsec` | Planned | IKEv2 daemon with kernel XFRM integration. |

## `zpldctl` Usage

```
zpldctl list                        List all workers and their current state
zpldctl status <id>                 Detailed status for one worker
zpldctl update worker <id>          Trigger a lazy update for a worker
zpldctl update supervisor           Trigger a lazy update of the supervisor
zpldctl cancel <id>                 Cancel a pending update
zpldctl add <config>                Spawn a new worker from a config file
zpldctl remove <id>                 Gracefully shut down and remove a worker
zpldctl events                      Stream health and lifecycle events
```

## Building

```bash
cargo build                         Build all crates
cargo build --release               Production build
cargo test                          Run all tests
```

## Testing

Worker integration tests run in Docker. Each worker directory contains a
`test/` folder with a `docker-compose.yml` that sets up the worker and any
peer processes needed to exercise it.

```bash
cd workers/udp-counter/test
docker compose up
```

## Status

Early development. Framework and the `udp-counter` reference worker are the
current focus. The IPsec worker follows once the framework is validated.
