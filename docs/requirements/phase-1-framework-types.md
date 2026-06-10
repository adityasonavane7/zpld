# Phase 1 — Framework Types and Traits

**Crate:** `zpld-framework`
**Status:** Done
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
- [x] Trait compiles with all listed methods
- [x] Trait is object-safe (confirmed by `dyn Worker` usage in `test_ft_fr_01_worker_trait_object_safe`)
- [x] All methods return `Result` or a concrete type (no panics in signatures)
- [x] A minimal no-op struct (`StubWorker`) can implement the trait with no errors

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
- [x] Struct compiles with all listed fields
- [x] All field types are defined within the framework crate
- [x] `WorkerContext` is not `Clone` (FDs should not be duplicated silently)

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
- [x] `StabilityStatus` enum has exactly `Stable` and `Unstable` variants
- [x] `StabilityCondition` struct compiles with all listed fields
- [x] A `Vec<StabilityCondition>` with mixed blocking/non-blocking conditions
      can be evaluated: patch is blocked iff any `blocking == true && Unstable`
      (verified by `is_patch_blocked` + tests `test_ft_fr_03_blocking_holds_patch`
      and `test_ft_fr_03_nonblocking_does_not_hold`)

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

> **Note:** `FdStore` exists as a typed stub. The `send`/`recv` method
> implementations require OS-level SCM_RIGHTS FD passing and are intentionally
> deferred to Phase 4 (Supervisor Core), where the full IPC infrastructure exists.

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
- [x] All variants compile
- [ ] `std::io::Error` can be converted via `?` without explicit wrapping
- [ ] Error messages include the variant context in their `Display` output

> **Note:** All variants are defined and compile. The `Display`,
> `std::error::Error`, and `From<io::Error>` implementations are deferred to
> Phase 4, where `WorkerError` is first used in real code and the implementations
> can be tested in context.

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
- [x] Enum compiles with all variants
- [x] Enum derives `Debug`, `Clone`, `PartialEq`
- [x] All transitions are representable (no variants that can't be reached)

---

## Non-Functional Requirements

### FT-NFR-01: No Std I/O in Types
**Priority:** P0 — Must Have
**Description:** Phase 1 types must not perform any I/O or system calls. All
types are pure data definitions. This keeps them testable in isolation and
usable in any async or sync context.

**Status:** Met. No I/O or system calls in any Phase 1 type.

### FT-NFR-02: Stable Public API
**Priority:** P0 — Must Have
**Description:** Once Phase 1 is merged, changes to the `Worker` trait or
`WorkerContext` that break existing implementations require a new major version
and a migration guide. Treat this as a public API from day one.

**Status:** Met. All types and the trait are `pub`. API is considered stable.

---

## Test Plan

### Unit Tests (in `crates/zpld-framework/src/lib.rs`)

| Test | Requirement | Status | Description |
|---|---|---|---|
| `test_ft_fr_01_worker_trait_object_safe` | FT-FR-01 | Pass | `Box<dyn Worker>` compiles with `StubWorker` |
| `test_ft_fr_03_blocking_holds_patch` | FT-FR-03 | Pass | Blocking+Unstable condition holds the patch |
| `test_ft_fr_03_nonblocking_does_not_hold` | FT-FR-03 | Pass | Non-blocking+Unstable does not hold the patch |
| `test_fdstore_recv_none_on_empty` | FT-FR-04 | Deferred | Requires Phase 4 FdStore implementation |
| `test_worker_error_display` | FT-FR-05 | Deferred | Requires Phase 4 Display impl |
| `test_worker_state_transitions` | FT-FR-06 | Not written | All state variants reachable and comparable |
