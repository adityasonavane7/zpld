# Phase 6 — Patch Scheduler

**Crate:** `zpld-framework`
**Status:** Not Started
**Design doc:** [docs/architecture.md](../architecture.md#lazy-update-lifecycle)

## Overview

The Patch Scheduler is the component that drives the lazy update lifecycle.
When an update is requested, it polls stability, coordinates the spawn of the
new worker, manages the drain and handoff sequence, and handles failures.
This is the core value of the framework.

## Dependencies

- Phase 1–4 complete
- Phase 5 complete (need a real worker to test against)

## Out of Scope

- Supervisor self-patching (Phase 8)
- Multi-worker coordinated updates

---

## Functional Requirements

### PS-FR-01: Update Request Handling
**Priority:** P0 — Must Have
**Description:** When the supervisor receives a `ControlRequest::UpdateWorker`
for a known worker ID, it queues an update for that worker by transitioning
its state to `UpdatePending`. Only one update per worker can be in progress
at a time.

**Acceptance Criteria:**
- [ ] `UpdatePending` state is set in registry and StateStore
- [ ] A second `UpdateWorker` for the same ID while `UpdatePending` returns
      `ControlResponse::Error("update already pending")`
- [ ] `UpdateWorker` for an unknown ID returns `ControlResponse::Error`

---

### PS-FR-02: Stability Polling
**Priority:** P0 — Must Have
**Description:** While a worker is `UpdatePending`, the supervisor periodically
sends `StabilityCheck` and waits for `StabilityReport`. If all blocking
conditions are `Stable`, the scheduler proceeds. If any blocking condition
is `Unstable`, it waits and retries after `stability_poll_interval_secs` (default 5s).

**Acceptance Criteria:**
- [ ] Polling continues until all blocking conditions are `Stable`
- [ ] Non-blocking `Unstable` conditions do not hold the patch
- [ ] `WorkerEvent::UpdateStarted` is not emitted until stable
- [ ] `zpldctl status` shows pending stability conditions during the wait

---

### PS-FR-03: New Worker Spawn (Shadow Mode)
**Priority:** P0 — Must Have
**Description:** Once stability is confirmed, the supervisor spawns the new
worker binary (from the `update_binary_path` field set by the operator) with
the same `--config` and `--worker-id` arguments. The new worker starts in
shadow mode — it reads StateStore and initializes, but does not yet serve traffic.

**Acceptance Criteria:**
- [ ] New binary is spawned and tracked separately from the old worker
- [ ] New worker's PID is recorded in the registry as `next_pid`
- [ ] Old worker continues running and heartbeating during this phase
- [ ] New worker failure to spawn → update aborted, old worker stays `Running`

---

### PS-FR-04: New Worker Readiness Wait
**Priority:** P0 — Must Have
**Description:** After spawning, the supervisor waits for `WorkerResponse::Ready`
from the new worker. If `Ready` is not received within `startup_timeout_secs`,
the new worker is killed and the update is aborted.

**Acceptance Criteria:**
- [ ] `Ready` received → proceed to drain
- [ ] Timeout → kill new worker, reset old worker to `Running`, emit `UpdateFailed`
- [ ] Timeout duration is configurable per worker (inherits supervisor default)

---

### PS-FR-05: Drain Old Worker
**Priority:** P0 — Must Have
**Description:** After the new worker is ready, the supervisor sends `Drain`
to the old worker and waits for `DrainAck`. The old worker must write its
final state to StateStore before returning `DrainAck`.

**Acceptance Criteria:**
- [ ] `Drain` is sent to old worker after new worker sends `Ready`
- [ ] Supervisor waits for `DrainAck` (timeout: `drain_timeout_secs`, default 30s)
- [ ] Drain timeout → abort update, new worker killed, old worker stays active
- [ ] `WorkerEvent::UpdateStarted` is emitted when drain begins

---

### PS-FR-06: FD Handoff
**Priority:** P0 — Must Have
**Description:** After `DrainAck`, the supervisor creates a temporary Unix
socket, sends `Handoff { fd_socket_path }` to the old worker, and instructs
the new worker to connect and receive FDs. The old worker sends FDs via
SCM_RIGHTS and returns `HandoffComplete`.

**Acceptance Criteria:**
- [ ] Temp socket is created at `/run/zpld/handoff-<worker-id>.sock`
- [ ] Old worker sends its FDs over the temp socket
- [ ] New worker receives FDs before old worker exits
- [ ] Temp socket is removed after handoff completes
- [ ] Handoff failure → both workers remain alive; update aborted

---

### PS-FR-07: Worker Promotion
**Priority:** P0 — Must Have
**Description:** After `HandoffComplete`, the supervisor kills the old worker
(SIGTERM, grace period 5s, then SIGKILL) and updates the registry: new PID,
new binary path, new version, state = `Running`.

**Acceptance Criteria:**
- [ ] Old worker receives SIGTERM after `HandoffComplete`
- [ ] Registry is updated atomically in StateStore
- [ ] `WorkerEvent::UpdateCompleted` is emitted with old and new versions
- [ ] `zpldctl list` shows the new version immediately after promotion

---

### PS-FR-08: Update Cancellation
**Priority:** P1 — Should Have
**Description:** `ControlRequest::CancelUpdate` cancels a pending update if
it has not yet reached the drain phase. Cancellation after drain has begun
is not supported and returns an error.

**Acceptance Criteria:**
- [ ] Cancel while `UpdatePending` (not yet draining) → state reset to `Running`
- [ ] Cancel after drain begun → `Error("cannot cancel: drain in progress")`
- [ ] Cancelled new worker (if already spawned) is killed

---

## Non-Functional Requirements

### PS-NFR-01: Old Worker Always Wins on Failure
**Priority:** P0 — Must Have
**Description:** Any failure during the update sequence (new worker crash,
timeout, FD handoff error) must leave the old worker running and serving
traffic. The old worker is never killed before the new worker is ready and
handoff is complete.

### PS-NFR-02: Atomic Registry Update
**Priority:** P0 — Must Have
**Description:** The transition from old to new worker in the registry must
be written to StateStore as a single locked operation. A supervisor crash
mid-update must leave the StateStore in a consistent, recoverable state.

---

## Test Plan

### Docker Integration Tests

| Test | Requirements | Description |
|---|---|---|
| Full lazy swap — happy path | PS-FR-01 through PS-FR-07 | Swap udp-counter; count continuous, no socket rebind |
| New worker fails readiness | PS-FR-04 | Broken binary; old worker stays up |
| Drain timeout | PS-FR-05 | Worker ignores drain; update aborts |
| FD handoff failure | PS-FR-06 | Temp socket removed; both workers survive |
| Cancel before drain | PS-FR-08 | Update cancelled; state reset to Running |
| Supervisor crash mid-update | PS-NFR-02 | StateStore remains consistent; old worker still alive |
