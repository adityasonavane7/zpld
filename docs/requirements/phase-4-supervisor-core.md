# Phase 4 — Supervisor Core

**Crate:** `zpld-framework` + `zpld-supervisor` binary
**Status:** Not Started
**Design doc:** [docs/architecture.md](../architecture.md)

## Overview

The supervisor is the host process for all workers. It reads configuration,
spawns worker binaries, monitors their health, and serves the control socket
for `zpldctl`. This phase covers everything except the patch lifecycle (Phase 6)
and self-patching (Phase 8).

## Dependencies

- Phase 1 complete (types and traits)
- Phase 2 complete (StateStore)
- Phase 3 complete (IPC wire format)

## Out of Scope

- Lazy update / patch scheduling (Phase 6)
- Supervisor self-patching (Phase 8)
- Worker-specific logic

---

## Functional Requirements

### SUP-FR-01: Config File Parsing
**Priority:** P0 — Must Have
**Description:** The supervisor binary reads a TOML config file at the path
given by `--config`. It must parse the supervisor section and all `[[worker]]`
blocks. Invalid config must produce a human-readable error and exit, not panic.

**Config fields — supervisor section:**
- `control_socket: String` — path to Unix socket for zpldctl
- `state_store: String` — path to StateStore file
- `heartbeat_interval_secs: u64`
- `heartbeat_timeout_secs: u64`

**Config fields — per worker block:**
- `id: String` — unique identifier
- `binary: String` — path to worker binary
- `config: String` — path to worker config file
- `auto_restart: bool`
- `update_policy: UpdatePolicy` — `lazy | immediate | manual`

**Acceptance Criteria:**
- [ ] Valid config is parsed without error
- [ ] Missing required fields produce a named error ("missing field: `id`")
- [ ] Unknown fields produce a warning, not an error
- [ ] `update_policy` parses case-insensitively

---

### SUP-FR-02: Worker Spawning
**Priority:** P0 — Must Have
**Description:** For each `[[worker]]` block in config, the supervisor spawns
the specified binary with `--config <path>` and `--worker-id <id>` arguments.
The child process's stdout and stderr are captured and forwarded to the
supervisor's log output with a `[worker-id]` prefix.

**Acceptance Criteria:**
- [ ] Binary is spawned with the correct arguments
- [ ] Supervisor does not exit if a worker fails to spawn; it logs the error
      and marks the worker `Dead`
- [ ] Spawned process is a child of the supervisor (tracked by PID)
- [ ] Worker stdout/stderr lines appear in supervisor logs with ID prefix

---

### SUP-FR-03: Worker Registry
**Priority:** P0 — Must Have
**Description:** The supervisor maintains a registry of all managed workers.
The registry is backed by the StateStore (persistent across restarts) and
a live in-memory view (for fast access).

**Operations required:**
- `register(config)` — add a new worker record
- `update_state(id, state)` — change a worker's `WorkerState`
- `update_pid(id, pid)` — update PID after spawn
- `lookup(id) -> Option<WorkerRecord>`
- `list() -> Vec<WorkerRecord>`
- `remove(id)` — mark slot as unoccupied

**Acceptance Criteria:**
- [ ] Registry is consistent between in-memory and StateStore after every write
- [ ] On supervisor restart, registry is rebuilt from StateStore
- [ ] Workers whose PIDs are no longer alive are marked `Dead` on rebuild

---

### SUP-FR-04: Health Monitor
**Priority:** P0 — Must Have
**Description:** The supervisor periodically sends `Heartbeat` to each worker
and expects a `HeartbeatAck` within `heartbeat_timeout_secs`. If no ack is
received, the worker is marked `Dead`. If `auto_restart = true`, the supervisor
respawns the worker.

**Acceptance Criteria:**
- [ ] Heartbeat is sent every `heartbeat_interval_secs`
- [ ] Missing ack within deadline marks worker `Dead` in registry and StateStore
- [ ] `WorkerEvent::HeartbeatMissed` is emitted on first missed heartbeat
- [ ] `WorkerEvent::WorkerDead` is emitted when marked Dead
- [ ] Auto-restart spawns a new process and registers it under the same ID
- [ ] Auto-restart backs off: 5s, 10s, 30s between attempts

---

### SUP-FR-05: Control Socket Server
**Priority:** P0 — Must Have
**Description:** The supervisor listens on a Unix socket at `control_socket`
for connections from `zpldctl`. Each connection receives one request and
returns one response (or a stream of `Event` responses for `StreamEvents`).
Connections are handled concurrently.

**Acceptance Criteria:**
- [ ] Socket is created at the configured path on startup
- [ ] Old socket file is removed if it exists from a previous run
- [ ] Multiple concurrent `zpldctl` connections are handled without blocking each other
- [ ] `List` returns the current registry state
- [ ] `Status { id }` returns `WorkerDetail` for a known ID; `Error` for unknown
- [ ] `StreamEvents` keeps the connection open and sends events as they occur
- [ ] Connection from unprivileged process is rejected (socket permissions: 0660, group: zpld)

---

### SUP-FR-06: Ready Signal Handling
**Priority:** P0 — Must Have
**Description:** When a worker sends `WorkerResponse::Ready`, the supervisor
transitions that worker from `Starting` to `Running` in the registry.
A worker that does not send `Ready` within a startup timeout is killed and
marked `Dead`.

**Acceptance Criteria:**
- [ ] `Ready` received → state transitions `Starting → Running`
- [ ] `Ready` not received within `startup_timeout_secs` (default 30s) → kill + `Dead`
- [ ] `WorkerEvent::WorkerStarted` is emitted on `Ready`

---

## Non-Functional Requirements

### SUP-NFR-01: Async Runtime
**Priority:** P0 — Must Have
**Description:** The supervisor uses `tokio` for all async I/O. The heartbeat
loop, control socket server, and worker communication are all async tasks.
No blocking calls on the async executor.

### SUP-NFR-02: Graceful Shutdown
**Priority:** P1 — Should Have
**Description:** On `SIGTERM`, the supervisor sends `Shutdown` to all workers,
waits up to 10 seconds for them to exit cleanly, then exits itself.

---

## Test Plan

### Integration Tests (in `crates/zpld-framework/tests/`)

| Test | Requirement | Description |
|---|---|---|
| `test_sup_fr_01_invalid_config_exits` | SUP-FR-01 | Bad TOML → non-zero exit, readable error |
| `test_sup_fr_02_worker_spawned_with_args` | SUP-FR-02 | Spawned binary receives `--worker-id` and `--config` |
| `test_sup_fr_03_registry_rebuilt_on_restart` | SUP-FR-03 | Restart supervisor, registry matches StateStore |
| `test_sup_fr_04_dead_worker_auto_restart` | SUP-FR-04 | Kill worker process; supervisor respawns it |
| `test_sup_fr_05_control_socket_list` | SUP-FR-05 | Connect, send `List`, receive `WorkerList` |
| `test_sup_fr_06_ready_timeout_kills_worker` | SUP-FR-06 | Worker never sends Ready; supervisor kills it |
