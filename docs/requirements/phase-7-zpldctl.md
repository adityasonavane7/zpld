# Phase 7 — zpldctl CLI

**Crate:** `zpldctl`
**Status:** Not Started

## Overview

`zpldctl` is the operator's interface to the supervisor. It connects to the
supervisor's Unix control socket, sends a request, prints the response, and
exits. The `events` command is the exception — it keeps the connection open
and streams events until interrupted.

## Dependencies

- Phase 3 complete (IPC wire format must be defined)
- Phase 4 complete (supervisor control socket must be running)

## Out of Scope

- TUI or interactive shell mode
- Authentication beyond socket file permissions
- Remote (non-local) connections

---

## Functional Requirements

### CTL-FR-01: Command Parsing
**Priority:** P0 — Must Have
**Description:** `zpldctl` uses `clap` for argument parsing. All commands and
their arguments are defined declaratively. `--help` and `--version` work
without a running supervisor.

**Top-level arguments:**
- `--socket <path>` — override control socket path (default: `/run/zpld/supervisor.sock`)

**Subcommands:**
- `list`
- `status <worker-id>`
- `update worker <worker-id>`
- `update supervisor`
- `cancel <worker-id>`
- `add <config-path>`
- `remove <worker-id>`
- `events`

**Acceptance Criteria:**
- [ ] `zpldctl --help` prints usage without connecting to socket
- [ ] Unknown subcommand prints usage and exits non-zero
- [ ] Missing required argument prints usage and exits non-zero

---

### CTL-FR-02: `list` Command
**Priority:** P0 — Must Have
**Description:** Prints a table of all workers with columns:
`ID`, `VERSION`, `STATE`, `HEALTH`, `UPTIME`.

**Acceptance Criteria:**
- [ ] Output is a fixed-width table readable in a terminal
- [ ] `STATE` reflects the current `WorkerState` value
- [ ] `HEALTH` is `OK`, `DEGRADED`, or `DEAD`
- [ ] Empty worker list prints headers and a "no workers" message

---

### CTL-FR-03: `status` Command
**Priority:** P0 — Must Have
**Description:** Prints detailed status for one worker. Output has two
sections: framework-level fields (ID, binary, version, state, health, uptime,
stability conditions) and a `[Worker Status]` section containing the
`status_blob` key-value pairs.

**Acceptance Criteria:**
- [ ] Framework fields are always shown, even if status_blob is empty
- [ ] `status_blob` keys are printed in sorted order
- [ ] Unknown worker ID prints an error message and exits non-zero
- [ ] Stability conditions list shows name, blocking flag, and reason if Unstable

---

### CTL-FR-04: `update worker` Command
**Priority:** P0 — Must Have
**Description:** Sends `UpdateWorker` to the supervisor and prints the
response. If the supervisor accepts the request, prints a confirmation and
the operator is informed to watch `zpldctl events`.

**Acceptance Criteria:**
- [ ] On success: prints `"Update queued for <id>. Watch with: zpldctl events"`
- [ ] On error (already pending, unknown ID): prints error message, exits non-zero
- [ ] Does not block waiting for the update to complete

---

### CTL-FR-05: `update supervisor` Command
**Priority:** P0 — Must Have
**Description:** Sends `UpdateSupervisor`. Prints a warning that the control
socket will briefly disconnect during the supervisor swap and asks for
confirmation unless `--yes` is passed.

**Acceptance Criteria:**
- [ ] Without `--yes`: prompts for confirmation before sending
- [ ] With `--yes`: sends immediately
- [ ] Prints expected disconnect warning

---

### CTL-FR-06: `cancel` Command
**Priority:** P1 — Should Have
**Description:** Sends `CancelUpdate` and prints the response.

**Acceptance Criteria:**
- [ ] Success: prints `"Update cancelled for <id>"`
- [ ] Error: prints error and exits non-zero

---

### CTL-FR-07: `events` Command
**Priority:** P0 — Must Have
**Description:** Sends `StreamEvents` and prints each incoming `WorkerEvent`
as a timestamped line until the user presses Ctrl-C or the connection drops.

**Output format per event:**
```
2026-06-08T14:23:01Z  [udp-counter-1]  UpdateCompleted  v0.1.0 → v0.2.0
2026-06-08T14:23:01Z  [udp-counter-1]  WorkerStarted    v0.2.0
```

**Acceptance Criteria:**
- [ ] Each event is on one line with ISO 8601 timestamp, worker ID, event type, and details
- [ ] Ctrl-C exits cleanly with exit code 0
- [ ] If the connection drops unexpectedly, prints an error and exits non-zero

---

### CTL-FR-08: Socket Connection Error Handling
**Priority:** P0 — Must Have
**Description:** If the control socket does not exist or the connection is
refused, `zpldctl` prints a clear message and exits non-zero. It does not
print a Rust backtrace or panic.

**Acceptance Criteria:**
- [ ] Missing socket: `"Cannot connect to supervisor: /run/zpld/supervisor.sock not found. Is zpld-supervisor running?"`
- [ ] Permission denied: `"Cannot connect to supervisor: permission denied. Are you in the 'zpld' group?"`
- [ ] Exit code is non-zero for all error conditions

---

## Non-Functional Requirements

### CTL-NFR-01: No Async Runtime
**Priority:** P1 — Should Have
**Description:** `zpldctl` makes one synchronous socket connection per invocation.
It does not need an async runtime. Use synchronous `std::os::unix::net::UnixStream`.
This keeps the binary small and startup fast.

### CTL-NFR-02: Exit Codes
**Priority:** P0 — Must Have

| Exit Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Supervisor returned an error response |
| `2` | Could not connect to supervisor |
| `3` | Invalid arguments |

---

## Test Plan

### Integration Tests (in `crates/zpldctl/tests/`)

| Test | Requirement | Description |
|---|---|---|
| `test_ctl_fr_01_help_no_socket` | CTL-FR-01 | `--help` works without supervisor |
| `test_ctl_fr_02_list_empty` | CTL-FR-02 | Supervisor with no workers → headers + "no workers" |
| `test_ctl_fr_02_list_with_workers` | CTL-FR-02 | Supervisor with workers → correct table |
| `test_ctl_fr_08_missing_socket` | CTL-FR-08 | No supervisor → clear message, exit 2 |
