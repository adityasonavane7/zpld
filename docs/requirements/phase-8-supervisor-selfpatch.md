# Phase 8 — Supervisor Self-Patch

**Crate:** `zpld-framework` + `zpld-supervisor`
**Status:** Not Started
**Design doc:** [docs/architecture.md](../architecture.md)

## Overview

The supervisor can patch itself without stopping its workers. It uses
`fork()` + `exec()` to start the new binary, waits for it to signal
readiness via the StateStore, and then exits. Workers are unaware the
supervisor changed — they reconnect to the new supervisor's control socket.

## Dependencies

- Phases 1–7 complete and validated
- Phase 6 complete (patch scheduler, workers must be patchable first)

## Out of Scope

- Patching supervisor and all workers simultaneously
- Rolling back a failed supervisor patch if the new binary is already running

---

## Functional Requirements

### SPT-FR-01: Update Request via zpldctl
**Priority:** P0 — Must Have
**Description:** `zpldctl update supervisor` sends `UpdateSupervisor` to
the running supervisor. The supervisor checks that no workers are currently
in a patch sequence (state ≠ `Running` for any worker) before proceeding.

**Acceptance Criteria:**
- [ ] If any worker is not `Running`: returns `Error("workers not stable; wait for updates to complete")`
- [ ] If all workers are `Running`: proceeds with self-patch
- [ ] `WorkerEvent::UpdateStarted` is emitted with `worker_id = "supervisor"`

---

### SPT-FR-02: Fork and Exec New Binary
**Priority:** P0 — Must Have
**Description:** The supervisor forks itself. The child process execs the
new supervisor binary (from a well-known path: `/usr/sbin/zpld-supervisor.new`
or a path specified in the update request). The parent waits for the child
to signal readiness before exiting.

**Acceptance Criteria:**
- [ ] `fork()` is called; child calls `exec()` with the new binary path
- [ ] Parent enters a wait loop, not a busy spin (uses a pipe or StateStore flag)
- [ ] If `exec()` fails (binary not found, not executable): parent logs error and continues running as if nothing happened
- [ ] The parent does not exit before the child signals ready

---

### SPT-FR-03: New Supervisor Startup
**Priority:** P0 — Must Have
**Description:** The new supervisor binary reads the StateStore, discovers
all existing workers (by their PIDs and socket paths), and reconnects to them
over their IPC sockets. It does not re-spawn workers that are already running.

**Acceptance Criteria:**
- [ ] New supervisor reads `WorkerRecord` for all occupied slots
- [ ] Workers whose PID is alive are connected to, not re-spawned
- [ ] Workers whose PID is dead are marked `Dead` (same as crash recovery in SUP-FR-03)
- [ ] New supervisor opens the control socket at the same path as the old one
      (old socket file is removed and recreated)

---

### SPT-FR-04: Readiness Signalling
**Priority:** P0 — Must Have
**Description:** The new supervisor signals readiness to the old supervisor
(parent) by writing a ready flag to the StateStore's supervisor record and
sending a byte on a pipe inherited across the fork.

**Acceptance Criteria:**
- [ ] Parent unblocks and exits only after receiving the readiness signal
- [ ] New supervisor's PID is written to the SupervisorRecord before signalling
- [ ] `WorkerEvent::UpdateCompleted` is emitted (by the new supervisor) after takeover

---

### SPT-FR-05: Workers Unaffected
**Priority:** P0 — Must Have
**Description:** During and after the supervisor self-patch, all workers must
continue running and heartbeating normally. No worker should receive a `Shutdown`
or `Drain` as part of the supervisor patch.

**Acceptance Criteria:**
- [ ] Workers do not restart during supervisor patch
- [ ] UDP packet count is continuous across the supervisor patch (Docker test)
- [ ] Worker IPC sockets remain open during the patch window

---

### SPT-FR-06: Failure Handling
**Priority:** P0 — Must Have
**Description:** If the new supervisor fails to signal ready within a timeout
(default 60s), the parent supervisor assumes the patch failed and continues
running. The orphaned child process is killed.

**Acceptance Criteria:**
- [ ] Timeout → parent kills child (SIGKILL), logs error, continues normally
- [ ] Workers are not affected by the failed patch
- [ ] `WorkerEvent::UpdateFailed` is emitted with `worker_id = "supervisor"`
- [ ] Operator can re-attempt the patch after failure

---

## Non-Functional Requirements

### SPT-NFR-01: No Double-Open of StateStore
**Priority:** P0 — Must Have
**Description:** Both old and new supervisor have the StateStore mmap'd
simultaneously during the handoff window. The process-shared mutex in
the StateStore header must correctly serialize their access. The new supervisor
must not truncate or re-initialize the file.

### SPT-NFR-02: Control Socket Downtime
**Priority:** P1 — Should Have
**Description:** The gap between the old supervisor removing the control socket
and the new supervisor creating it should be under 100ms. `zpldctl` connections
during this window receive a connection refused error and should retry.

---

## Test Plan

### Docker Integration Tests

| Test | Requirements | Description |
|---|---|---|
| Full supervisor swap | SPT-FR-01 to SPT-FR-05 | Swap supervisor while udp-counter runs; count continuous |
| New binary not found | SPT-FR-02, SPT-FR-06 | Bad binary path; old supervisor stays up |
| New supervisor hangs | SPT-FR-06 | Child never signals ready; parent kills it and continues |
| Concurrent worker patch + supervisor patch | SPT-FR-01 | Supervisor refuses self-patch while worker update is in progress |
