# Phase 2 — StateStore

**Crate:** `zpld-framework`
**Status:** Not Started
**Design doc:** [docs/state-store.md](../state-store.md)

## Overview

The StateStore is a memory-mapped file shared between the supervisor and all
workers. It is the only state that survives a process restart. Both the
supervisor and workers read and write it. Making it correct and robust is
critical — corruption here affects every component.

## Dependencies

- Phase 1 complete (`WorkerError` type must exist)

## Out of Scope

- Worker-specific state structs (defined per worker)
- StateStore cleanup on shutdown (handled by OS: file lives in `/run`)

---

## Functional Requirements

### SS-FR-01: File Creation and Mapping
**Priority:** P0 — Must Have
**Description:** The `StateStore` must create the file at a configured path
if it does not exist and `mmap` it into process memory. If the file already
exists (supervisor restart after crash), it must map it without truncating.

**Acceptance Criteria:**
- [ ] File is created with correct size on first open
- [ ] Existing file is mapped without data loss on reopen
- [ ] File path is configurable, not hardcoded
- [ ] Returns a typed error if the file cannot be created or mapped

---

### SS-FR-02: Header Record
**Priority:** P0 — Must Have
**Description:** The first bytes of the StateStore hold a fixed-size `Header`
that contains a magic number for sanity checking, a format version, and a
process-shared mutex for coordinating concurrent access.

**Fields required:**
- `magic: u32` — fixed value `0x5A504C44` ("ZPLD"). Validates file is ours.
- `format_version: u32` — StateStore layout version. Currently `1`.
- `lock: pthread_mutex_t` — `PTHREAD_PROCESS_SHARED` mutex

**Acceptance Criteria:**
- [ ] On open, magic is checked. Wrong magic returns an error.
- [ ] On first create, magic and format version are written correctly
- [ ] Mutex is initialized with `PTHREAD_PROCESS_SHARED` on file creation

---

### SS-FR-03: Supervisor Record
**Priority:** P0 — Must Have
**Description:** A fixed-size slot after the header holds the supervisor's
own record, used by workers and by a new supervisor after self-patch.

**Fields required:**
- `pid: u32`
- `version: String` — binary version
- `control_socket_path: String`

**Acceptance Criteria:**
- [ ] Supervisor writes its record on startup
- [ ] A newly started supervisor reads this and can detect a running predecessor
- [ ] Record survives supervisor restart (written to file, not memory)

---

### SS-FR-04: Worker Record Slots
**Priority:** P0 — Must Have
**Description:** After the supervisor record, the file contains a fixed-size
table of worker record slots. Maximum number of workers is `MAX_WORKERS = 64`.
Each slot is addressed by index; the supervisor assigns indices from the config.

**Fields required per slot:**
- `occupied: bool` — whether this slot is in use
- `worker_id: String`
- `pid: u32`
- `binary_path: String`
- `version: String`
- `ipc_socket_path: String`
- `state: WorkerState`
- `last_heartbeat_secs: u64` — Unix timestamp

**Acceptance Criteria:**
- [ ] Slots are fixed-size; offset of slot N is deterministic without parsing
- [ ] `occupied = false` slots are treated as empty by all readers
- [ ] All 64 slots can be written and read back correctly

---

### SS-FR-05: Serialization with bincode
**Priority:** P0 — Must Have
**Description:** Variable-length fields within records (strings, protocol state
blobs) are serialized with `bincode`. Each serialized payload is preceded by a
`RecordHeader` containing a version field and byte length.

**RecordHeader fields:**
- `magic: u32` — per-record magic (`0x5A504C44`)
- `version: u32` — payload format version
- `length: u32` — byte length of the serialized payload

**Acceptance Criteria:**
- [ ] Any record can be serialized to bytes and deserialized back to the same value
- [ ] `RecordHeader` is always written before the payload
- [ ] A record with a corrupt `magic` returns a typed error on read

---

### SS-FR-06: Version Migration
**Priority:** P0 — Must Have
**Description:** When a binary reads a record with a lower version than
expected, it runs an upgrade function to migrate the data. Records with a
higher version than expected return an error. The migration infrastructure
must be in place even if the first version has no migrations yet.

**Acceptance Criteria:**
- [ ] A v1 record written by an old binary can be read by a v2 binary
      after implementing the migration function
- [ ] A record with version > current binary version returns `UnknownVersion`
- [ ] Migration is tested with an actual struct field addition

---

### SS-FR-07: Process-Shared Locking
**Priority:** P0 — Must Have
**Description:** All reads and writes to the StateStore must be protected by
the process-shared mutex in the Header. The `StateStore` API must make it
impossible to read or write without holding the lock.

**Acceptance Criteria:**
- [ ] Public read/write methods acquire the lock before any data access
- [ ] Lock is released on drop (RAII guard)
- [ ] Two separate processes can safely write to different slots concurrently

---

## Non-Functional Requirements

### SS-NFR-01: No Heap Allocation in Hot Path
**Priority:** P1 — Should Have
**Description:** Heartbeat writes (the most frequent StateStore operation)
should not allocate on the heap. Fixed-size serialization of the heartbeat
timestamp field must not require `Vec` allocation.

### SS-NFR-02: Explicit Error on Corruption
**Priority:** P0 — Must Have
**Description:** Any read that detects corruption (bad magic, length mismatch,
deserialization failure) must return a typed error. It must never silently
return zeroed or garbage data.

---

## Test Plan

### Unit Tests (in `crates/zpld-framework/src/state/`)

| Test | Requirement | Description |
|---|---|---|
| `test_ss_fr_01_creates_file_on_first_open` | SS-FR-01 | File does not exist; open creates it |
| `test_ss_fr_01_reopens_without_truncation` | SS-FR-01 | Write a record, reopen, data still present |
| `test_ss_fr_02_magic_check_on_open` | SS-FR-02 | Wrong magic → typed error |
| `test_ss_fr_05_roundtrip_serialization` | SS-FR-05 | Serialize → write → read → deserialize = same value |
| `test_ss_fr_06_migration_v1_to_v2` | SS-FR-06 | v1 bytes → v2 struct with default new field |
| `test_ss_fr_06_unknown_version_error` | SS-FR-06 | Version > current → `UnknownVersion` error |
| `test_ss_fr_07_lock_acquired_on_write` | SS-FR-07 | Concurrent writes to different slots do not corrupt |
