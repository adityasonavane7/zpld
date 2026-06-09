# zpld — Roadmap

## Vision

zpld is a framework for building live-patchable network daemons. A daemon built
on zpld can be updated — binary replaced, restarted — without dropping a single
packet, resetting a session, or causing a peer to notice. The kernel handles
traffic; zpld handles the rest.

The first target is an IPsec/IKEv2 daemon. The framework is protocol-agnostic.

---

## How This Roadmap Works

Each phase has a requirements document in `docs/requirements/`. Requirements are
numbered (e.g. `FT-FR-01`) so they can be referenced in commits, PRs, and test
names. Acceptance criteria define exactly when a requirement is met.

**Priority levels:**
- `P0 — Must Have`: Blocking. Phase is not complete without it.
- `P1 — Should Have`: Important. Included in this phase unless explicitly deferred.
- `P2 — Nice to Have`: Non-blocking. Documented for future phases.

**Status values:** `Not Started` → `In Progress` → `In Review` → `Done`

Once the GitHub repository is live, each requirement maps to one or more GitHub
Issues. The requirement doc is the source of truth; Issues are the work tracker.

---

## Phases

| # | Phase | Status | Requirements |
|---|---|---|---|
| 1 | Framework Types and Traits | Not Started | [docs/requirements/phase-1-framework-types.md](docs/requirements/phase-1-framework-types.md) |
| 2 | StateStore | Not Started | [docs/requirements/phase-2-state-store.md](docs/requirements/phase-2-state-store.md) |
| 3 | IPC Wire Format | Not Started | [docs/requirements/phase-3-ipc-protocol.md](docs/requirements/phase-3-ipc-protocol.md) |
| 4 | Supervisor Core | Not Started | [docs/requirements/phase-4-supervisor-core.md](docs/requirements/phase-4-supervisor-core.md) |
| 5 | UDP Counter Worker | Not Started | [docs/requirements/phase-5-udp-counter.md](docs/requirements/phase-5-udp-counter.md) |
| 6 | Patch Scheduler | Not Started | [docs/requirements/phase-6-patch-scheduler.md](docs/requirements/phase-6-patch-scheduler.md) |
| 7 | zpldctl CLI | Not Started | [docs/requirements/phase-7-zpldctl.md](docs/requirements/phase-7-zpldctl.md) |
| 8 | Supervisor Self-Patch | Not Started | [docs/requirements/phase-8-supervisor-selfpatch.md](docs/requirements/phase-8-supervisor-selfpatch.md) |

---

## Milestone: Framework Validated (Phases 1–7)

The framework is considered validated when:
- A udp-counter worker can be swapped live with zero packet loss in Docker
- The packet count is continuous across the swap (state survived)
- The UDP socket FD was passed to the new worker (no rebind)
- `zpldctl` correctly reports state before, during, and after the swap
- A StateStore migration (v1 → v2 struct) is tested and passes

This milestone gates work on the IPsec worker.

---

## Milestone: IPsec Worker (Post-Framework)

Planned. Depends on framework being validated.

- IKEv2 state machine
- Kernel XFRM netlink integration
- IPsec-specific stability conditions
- Live tunnel swap tested against a real peer

---

## Key Design Decisions (Locked)

These are not open for reconsideration without a written rationale and team
agreement. They are recorded here to prevent scope creep and revisiting.

| Decision | Choice | Rationale |
|---|---|---|
| Language | Rust | Zero GC pauses, memory safety, strong systems primitives |
| Process model | Supervisor + Workers | Both independently patchable |
| State persistence | mmap'd file + bincode | External to any process, fast, no extra dependencies |
| FD passing | SCM_RIGHTS over Unix socket | Standard POSIX, kernel-level, zero-copy |
| IPC serialization | bincode + length prefix | Fast, binary, Rust-native |
| Patch trigger | External CLI (zpldctl) | Operator controls timing |
| Update policy | Lazy (stability-gated) | Never patch during protocol instability |
| First worker | udp-counter (reference) | Validates framework before hard problems |
| Target daemon | IPsec / IKEv2 | Kernel XFRM handles dataplane; IKE is the patchable part |

---

## Testing Strategy

| Level | Tool | When |
|---|---|---|
| Unit | `cargo test` | Every commit |
| Integration | `cargo test` (tests/ dir) | Every PR |
| System | Docker Compose | Per phase milestone |
| Regression | Docker Compose | Before any merge to main |

All tests run in Docker or in-process. Nothing is tested on the host machine.
