# Phase 5 — UDP Counter Worker

**Crate:** `workers/udp-counter`
**Status:** Not Started
**Reference:** [workers/udp-counter/README.md](../../workers/udp-counter/README.md)

## Overview

The first complete worker implementation. Its protocol is trivial (count UDP
packets) so all complexity is in the framework mechanics: StateStore handoff,
FD passing, lifecycle implementation. This worker is both the test vehicle for
the framework and the reference for anyone writing future workers.

## Dependencies

- Phase 1 complete (Worker trait must exist to implement)
- Phase 2 complete (StateStore must be writable)
- Phase 3 complete (IPC messages must be encodable)
- Phase 4 complete (Supervisor must be able to spawn and manage the worker)

## Out of Scope

- Any network protocol beyond UDP receive
- Encryption, authentication
- Rate limiting or traffic shaping

---

## Functional Requirements

### UDP-FR-01: Config File Parsing
**Priority:** P0 — Must Have
**Description:** The worker reads a TOML config file at `--config`. Invalid
config produces a human-readable error and exits with non-zero status.

**Config fields:**
- `listen_addr: SocketAddr` — address and port to bind (e.g. `0.0.0.0:9000`)
- `source_filter: Option<IpAddr>` — if set, only count packets from this source
- `heartbeat_interval_secs: u64`

**Acceptance Criteria:**
- [ ] Valid config is parsed without error
- [ ] Missing `listen_addr` produces a named error
- [ ] `source_filter` absent means all sources are counted

---

### UDP-FR-02: UDP Socket Binding
**Priority:** P0 — Must Have
**Description:** On startup, if no predecessor FD is available in `FdStore`,
the worker binds a UDP socket to `listen_addr`. The socket must be set to
non-blocking mode.

**Acceptance Criteria:**
- [ ] Socket binds successfully to the configured address
- [ ] Socket is non-blocking
- [ ] Bind failure (address in use, permission denied) returns a typed error
      via `Worker::init`, not a panic

---

### UDP-FR-03: Packet Counting Loop
**Priority:** P0 — Must Have
**Description:** The worker receives UDP datagrams in an async loop. For each
datagram, if `source_filter` is set and the source IP matches, or if no filter
is set, increment `packet_count` and `byte_count`.

**Acceptance Criteria:**
- [ ] `packet_count` increments by 1 per qualifying datagram
- [ ] `byte_count` increments by the datagram payload length
- [ ] Datagrams from non-matching sources are silently discarded (not counted)
- [ ] The loop does not block the tokio executor (uses `tokio::net::UdpSocket`)

---

### UDP-FR-04: Worker Trait — init
**Priority:** P0 — Must Have
**Description:** Implement `Worker::init`. Must:
1. Parse config from `ctx.config_path`
2. Attempt to receive a UDP socket FD from `ctx.fd_store` (predecessor handoff)
3. If FD received: reconstruct `UdpSocket` from it
4. If no FD: bind a new socket
5. Read `WorkerState` from `ctx.state_store` (predecessor's packet count)
6. If state found: set `packet_count = state.packet_count`, `byte_count = state.byte_count`
7. If no state: start from zero
8. Start the packet counting loop

**Acceptance Criteria:**
- [ ] First startup (no predecessor): binds new socket, starts from zero
- [ ] After swap: inherits socket FD, continues count from predecessor's value
- [ ] Count is continuous across swap (no reset to zero)

---

### UDP-FR-05: Worker Trait — heartbeat
**Priority:** P0 — Must Have
**Description:** Implement `Worker::heartbeat`. Writes current `WorkerState`
to `ctx.state_store`. This ensures the state is always fresh even if
`drain` is never called (e.g. unexpected death).

**WorkerState fields:**
- `version: u32` — currently `1`
- `packet_count: u64`
- `byte_count: u64`
- `listen_addr: String`
- `source_filter: Option<String>`

**Acceptance Criteria:**
- [ ] `heartbeat` writes current counts to StateStore
- [ ] A new worker started after a crash reads the last heartbeat counts

---

### UDP-FR-06: Worker Trait — stability
**Priority:** P0 — Must Have
**Description:** Implement `Worker::stability`. The UDP counter has no
blocking stability conditions — it is always patchable.

**Acceptance Criteria:**
- [ ] Returns an empty `Vec` (no conditions) or a single non-blocking
      `Stable` condition for documentation purposes
- [ ] Never returns a blocking `Unstable` condition

---

### UDP-FR-07: Worker Trait — drain
**Priority:** P0 — Must Have
**Description:** Implement `Worker::drain`. Stop accepting new packets
(but do not close the socket) and write a final `WorkerState` to StateStore.

**Acceptance Criteria:**
- [ ] Final state is written to StateStore before `drain` returns
- [ ] Socket is not closed (will be passed via FdStore)
- [ ] Returns `Ok` after state is written

---

### UDP-FR-08: Worker Trait — handoff
**Priority:** P0 — Must Have
**Description:** Implement `Worker::handoff`. Send the UDP socket FD to the
new worker via `store.send("udp_socket", fd)`.

**Acceptance Criteria:**
- [ ] Socket FD is sent via `FdStore` under the key `"udp_socket"`
- [ ] Worker exits cleanly after `handoff` returns

---

### UDP-FR-09: status_blob
**Priority:** P0 — Must Have
**Description:** Implement `Worker::status_blob`. Returns a `HashMap` of
human-readable fields for display by `zpldctl status`.

**Required keys:** `listen_addr`, `source_filter` (or `"all"` if unset),
`packet_count`, `byte_count`

**Acceptance Criteria:**
- [ ] All four keys are always present
- [ ] `packet_count` and `byte_count` are formatted with comma separators
- [ ] `source_filter` shows `"all"` when no filter is configured

---

### UDP-FR-10: State Migration (v1 → v2)
**Priority:** P1 — Should Have
**Description:** As a deliberate test of the StateStore migration
infrastructure, implement a v1 → v2 migration by adding a `last_swap_time`
field to `WorkerState`. The migration must be tested end-to-end.

**Acceptance Criteria:**
- [ ] v2 binary reads a v1 StateStore and produces a valid v2 state
      (with `last_swap_time` defaulting to `0`)
- [ ] v2 binary writes v2 state correctly
- [ ] Test covers the full swap: write v1 → start v2 → verify count is continuous

---

## Non-Functional Requirements

### UDP-NFR-01: No Packet Loss During Handoff
**Priority:** P0 — Must Have
**Description:** The UDP socket is handed off via FdStore (SCM_RIGHTS). The
kernel socket buffer holds packets during the handoff window. The new worker
must drain the socket buffer after receiving the FD before processing new
packets. No packets should be counted twice or missed.

---

## Test Plan

### Unit Tests (in `workers/udp-counter/src/`)

| Test | Requirement | Description |
|---|---|---|
| `test_udp_fr_01_invalid_config` | UDP-FR-01 | Missing listen_addr → typed error |
| `test_udp_fr_05_state_roundtrip` | UDP-FR-05 | Serialize WorkerState v1 → deserialize → same values |
| `test_udp_fr_10_migration_v1_to_v2` | UDP-FR-10 | v1 bytes → v2 struct with default last_swap_time |

### Docker Integration Tests (in `workers/udp-counter/docker/`)

| Test | Requirements | Description |
|---|---|---|
| Worker swap — count continuous | UDP-FR-04, FR-08 | Send packets, swap worker, count does not reset |
| Worker swap — no socket rebind | UDP-FR-02, FR-08 | Netstat shows same socket port after swap |
| Crash recovery — last heartbeat count | UDP-FR-05 | Kill worker, restart, count from last heartbeat |
| State migration v1 → v2 | UDP-FR-10 | Old binary writes v1, new binary reads and migrates |
