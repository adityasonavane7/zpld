# Phase 3 — IPC Wire Format

**Crate:** `zpld-framework`
**Status:** Not Started
**Design doc:** [docs/ipc-protocol.md](../ipc-protocol.md)

## Overview

Define the wire format for all IPC channels in the system. There are two
channels: supervisor ↔ zpldctl (control protocol) and supervisor ↔ worker
(worker protocol). Both use Unix sockets with length-prefixed bincode messages.
This phase is purely about encoding/decoding — no sockets, no async.

## Dependencies

- Phase 1 complete (`StabilityCondition`, `WorkerState` types must exist)

## Out of Scope

- Actual socket I/O (that is Phase 4)
- Authentication or encryption of IPC channels
- Cross-host IPC

---

## Functional Requirements

### IPC-FR-01: Message Framing
**Priority:** P0 — Must Have
**Description:** All messages on both channels use identical framing: a
big-endian `u32` length prefix followed by that many bytes of bincode payload.
The frame functions must be usable with any `AsyncRead`/`AsyncWrite` or
synchronous `Read`/`Write`.

**Acceptance Criteria:**
- [ ] `encode_message(msg) -> Vec<u8>` produces a valid framed message
- [ ] `decode_message(bytes) -> Result<T>` correctly parses a framed message
- [ ] A message encoded and then decoded equals the original value
- [ ] A truncated frame returns a typed error, not a panic

---

### IPC-FR-02: Control Request Messages (zpldctl → Supervisor)
**Priority:** P0 — Must Have
**Description:** Define the `ControlRequest` enum covering all commands
`zpldctl` can send to the supervisor.

**Variants required:**
- `List` — request list of all workers
- `Status { worker_id: String }` — detailed status for one worker
- `UpdateWorker { worker_id: String }` — trigger lazy update
- `UpdateSupervisor` — trigger supervisor self-patch
- `CancelUpdate { worker_id: String }` — cancel pending update
- `AddWorker { config_path: String }` — spawn a new worker at runtime
- `RemoveWorker { worker_id: String }` — gracefully remove a worker
- `StreamEvents` — open a persistent event stream

**Acceptance Criteria:**
- [ ] All variants compile and derive `Serialize`/`Deserialize`
- [ ] Each variant round-trips through encode → decode correctly
- [ ] `StreamEvents` is distinguished from single-response commands at the type level

---

### IPC-FR-03: Control Response Messages (Supervisor → zpldctl)
**Priority:** P0 — Must Have
**Description:** Define `ControlResponse` and the supporting structs
`WorkerSummary` and `WorkerDetail`.

**Variants required:**
- `WorkerList(Vec<WorkerSummary>)`
- `WorkerStatus(WorkerDetail)`
- `Ok`
- `Error(String)`
- `Event(WorkerEvent)` — for `StreamEvents` responses

**WorkerSummary fields:** `id`, `binary_path`, `version`, `state: WorkerState`, `health: WorkerHealth`

**WorkerDetail fields:** `summary: WorkerSummary`, `stability: Vec<StabilityCondition>`,
`status_blob: HashMap<String, String>`

**Acceptance Criteria:**
- [ ] All variants and structs compile with `Serialize`/`Deserialize`
- [ ] `WorkerDetail.status_blob` is a passthrough — framework does not parse it
- [ ] Each variant round-trips through encode → decode correctly

---

### IPC-FR-04: Worker Health Type
**Priority:** P0 — Must Have
**Description:** Define `WorkerHealth` used in `WorkerSummary`.

**Variants:** `Ok`, `Degraded(String)`, `Dead`

**Acceptance Criteria:**
- [ ] Derives `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`
- [ ] `Dead` is only set when the process has exited or missed heartbeat deadline

---

### IPC-FR-05: Supervisor Command Messages (Supervisor → Worker)
**Priority:** P0 — Must Have
**Description:** Define `SupervisorCommand` — messages the supervisor sends
to a running worker.

**Variants required:**
- `Heartbeat` — request a heartbeat acknowledgement
- `StabilityCheck` — request current stability conditions
- `Drain` — begin drain phase
- `Handoff { fd_socket_path: String }` — connect to this socket and send/recv FDs
- `Shutdown` — clean exit

**Acceptance Criteria:**
- [ ] All variants compile with `Serialize`/`Deserialize`
- [ ] Each variant round-trips correctly

---

### IPC-FR-06: Worker Response Messages (Worker → Supervisor)
**Priority:** P0 — Must Have
**Description:** Define `WorkerResponse` — messages a worker sends back.

**Variants required:**
- `HeartbeatAck`
- `StabilityReport(Vec<StabilityCondition>)`
- `DrainAck`
- `HandoffComplete`
- `Ready` — sent once at startup when worker is fully operational
- `StatusBlob(HashMap<String, String>)`
- `Error(String)`

**Acceptance Criteria:**
- [ ] All variants compile with `Serialize`/`Deserialize`
- [ ] `Ready` is distinct from `HeartbeatAck` at the type level
- [ ] Each variant round-trips correctly

---

### IPC-FR-07: WorkerEvent Type
**Priority:** P1 — Should Have
**Description:** Define `WorkerEvent` for the supervisor's event stream,
consumed by `zpldctl events`.

**Variants required:**
- `WorkerStarted { worker_id: String, version: String }`
- `WorkerDead { worker_id: String, reason: String }`
- `UpdateStarted { worker_id: String }`
- `UpdateCompleted { worker_id: String, old_version: String, new_version: String }`
- `UpdateFailed { worker_id: String, reason: String }`
- `HeartbeatMissed { worker_id: String }`

**Acceptance Criteria:**
- [ ] All variants compile with `Serialize`/`Deserialize`
- [ ] Each round-trips correctly

---

## Non-Functional Requirements

### IPC-NFR-01: No Panics in Decode Path
**Priority:** P0 — Must Have
**Description:** Decoding a malformed or truncated message must return a
typed error. It must never panic. Fuzz-testing is encouraged on the decode path.

---

## Test Plan

### Unit Tests (in `crates/zpld-framework/src/ipc/`)

| Test | Requirement | Description |
|---|---|---|
| `test_ipc_fr_01_frame_roundtrip` | IPC-FR-01 | encode → decode = original |
| `test_ipc_fr_01_truncated_frame_error` | IPC-FR-01 | Truncated input → typed error, no panic |
| `test_ipc_fr_02_all_control_requests_roundtrip` | IPC-FR-02 | One test per variant |
| `test_ipc_fr_03_all_control_responses_roundtrip` | IPC-FR-03 | One test per variant |
| `test_ipc_fr_05_all_supervisor_commands_roundtrip` | IPC-FR-05 | One test per variant |
| `test_ipc_fr_06_all_worker_responses_roundtrip` | IPC-FR-06 | One test per variant |
