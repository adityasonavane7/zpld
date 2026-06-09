# Phase 1 — Framework Types and Traits

**Crate:** `zpld-framework`
**Status:** Not Started
**Design doc:** [docs/worker-contract.md](../worker-contract.md)

## Overview

Define all types and traits that form the public contract of `zpld-framework`.
This is pure Rust — no system calls, no I/O, no async. It establishes the
language every other component speaks. Everything in Phases 2–8 depends on
these definitions being stable.

## Dependencies

None. This is the foundation.

## Out of Scope

- Actual implementations of any trait (that is Phase 4–5)
- Serialization of types (that is Phase 2)
- Async runtime or I/O

---

## Functional Requirements

### FT-FR-01: Worker Trait
**Priority:** P0 — Must Have
**Description:** Define a `Worker` trait that any worker binary must implement
to participate in the zpld framework. The trait must be object-safe so the
supervisor can hold a `Box<dyn Worker>` without knowing the concrete type.

**Methods required:**
- `init(ctx: WorkerContext) -> Result<(), WorkerError>` — called once at startup
- `heartbeat() -> Result<(), WorkerError>` — called on a periodic timer
- `stability() -> Vec<StabilityCondition>` — returns current patch-readiness
- `drain() -> Result<(), WorkerError>` — called before patch; stop new ops
- `handoff(store: &mut FdStore) -> Result<(), WorkerError>` — pass FDs to successor
- `status_blob() -> HashMap<String, String>` — arbitrary key-value for zpldctl

**Acceptance Criteria:**
- [ ] Trait compiles with all listed methods
- [ ] Trait is object-safe (confirmed by `dyn Worker` usage in a test)
- [ ] All methods return `Result` or a concrete type (no panics in signatures)
- [ ] A minimal no-op struct can implement the trait with no errors

---

### FT-FR-02: WorkerContext
**Priority:** P0 — Must Have
**Description:** Define a `WorkerContext` struct passed by the framework to
`Worker::init`. Contains everything a worker needs to start up.

**Fields required:**
- `worker_id: String` — the ID from `supervisor.toml`
- `config_path: PathBuf` — path to the worker's config file
- `state_store: StateStoreSlot` — access to this worker's StateStore slot
- `fd_store: FdStore` — FDs inherited from the predecessor (empty on first start)

**Acceptance Criteria:**
- [ ] Struct compiles with all listed fields
- [ ] All field types are defined within the framework crate
- [ ] `WorkerContext` is not `Clone` (FDs should not be duplicated silently)

---

### FT-FR-03: StabilityCondition
**Priority:** P0 — Must Have
**Description:** Define the type that workers use to report their patch-readiness.
The supervisor evaluates these before initiating a patch.

**Fields required:**
- `name: String` — human-readable name shown in `zpldctl status`
- `blocking: bool` — if `true`, an `Unstable` status holds the patch
- `status: StabilityStatus` — `Stable` or `Unstable`
- `reason: Option<String>` — explanation shown when `Unstable`

**Acceptance Criteria:**
- [ ] `StabilityStatus` enum has exactly `Stable` and `Unstable` variants
- [ ] `StabilityCondition` struct compiles with all listed fields
- [ ] A `Vec<StabilityCondition>` with mixed blocking/non-blocking conditions
      can be evaluated: patch is blocked iff any `blocking == true && Unstable`

---

### FT-FR-04: FdStore
**Priority:** P0 — Must Have
**Description:** Define an `FdStore` type that abstracts sending and receiving
raw file descriptors between the old and new worker during a handoff.

**Methods required:**
- `send(name: &str, fd: RawFd) -> Result<(), WorkerError>` — called by old worker
- `recv(name: &str) -> Result<Option<RawFd>, WorkerError>` — called by new worker

`recv` returns `None` when no predecessor exists (first startup).

**Acceptance Criteria:**
- [ ] `FdStore` compiles with `send` and `recv`
- [ ] `recv` with a name that was never `send`-ed returns `Ok(None)`
- [ ] Type does not expose raw FD integers in its public API beyond these methods

---

### FT-FR-05: WorkerError
**Priority:** P0 — Must Have
**Description:** Define the error type returned by all `Worker` trait methods.
Must be usable with the `?` operator and carry enough context to log.

**Variants required:**
- `Init(String)` — failure during `init`
- `Heartbeat(String)` — failure during `heartbeat`
- `Drain(String)` — failure during `drain`
- `Handoff(String)` — failure during `handoff`
- `Io(std::io::Error)` — wraps a standard I/O error

**Acceptance Criteria:**
- [ ] `WorkerError` implements `std::error::Error` and `Display`
- [ ] All variants compile
- [ ] `std::io::Error` can be converted via `?` without explicit wrapping
- [ ] Error messages include the variant context in their `Display` output

---

### FT-FR-06: WorkerState Enum
**Priority:** P0 — Must Have
**Description:** Define the lifecycle state of a worker as seen by the supervisor.
Used in the registry and reported by `zpldctl list`.

**Variants required:**
- `Starting` — spawned, not yet signalled ready
- `Running` — operational, heartbeating normally
- `UpdatePending` — update queued, waiting for stability
- `Draining` — drain command sent, waiting for completion
- `Dead` — process has exited or missed heartbeat deadline

**Acceptance Criteria:**
- [ ] Enum compiles with all variants
- [ ] Enum derives `Debug`, `Clone`, `PartialEq`
- [ ] All transitions are representable (no variants that can't be reached)

---

## Non-Functional Requirements

### FT-NFR-01: No Std I/O in Types
**Priority:** P0 — Must Have
**Description:** Phase 1 types must not perform any I/O or system calls. All
types are pure data definitions. This keeps them testable in isolation and
usable in any async or sync context.

### FT-NFR-02: Stable Public API
**Priority:** P0 — Must Have
**Description:** Once Phase 1 is merged, changes to the `Worker` trait or
`WorkerContext` that break existing implementations require a new major version
and a migration guide. Treat this as a public API from day one.

---

## Test Plan

### Unit Tests (in `crates/zpld-framework/src/`)

| Test | Requirement | Description |
|---|---|---|
| `test_worker_trait_object_safe` | FT-FR-01 | `Box<dyn Worker>` compiles with a stub impl |
| `test_stability_blocking_holds_patch` | FT-FR-03 | Mixed conditions: patch held when any blocking is Unstable |
| `test_stability_nonblocking_does_not_hold` | FT-FR-03 | Non-blocking Unstable does not hold the patch |
| `test_fdstore_recv_none_on_empty` | FT-FR-04 | `recv` returns `Ok(None)` with no predecessor |
| `test_worker_error_display` | FT-FR-05 | Each variant produces a non-empty Display string |
| `test_worker_state_transitions` | FT-FR-06 | All state variants are reachable and comparable |
