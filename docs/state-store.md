# StateStore

The StateStore is a memory-mapped file at `/run/zpld/state.db`. It is the
single source of truth that makes both the supervisor and workers live-patchable.
Because state is external to any process, any process can be replaced and the
replacement reads full context immediately.

## File Layout

```
Offset 0:   Header          (fixed size, version + global lock)
Offset 128: SupervisorRecord (fixed size)
Offset 256: WorkerRecord[0]
Offset 256 + WORKER_SLOT_SIZE * 1: WorkerRecord[1]
...
```

Sizes are fixed to allow direct offset calculation without parsing the file
from the beginning. The maximum number of workers per supervisor is defined
at compile time (`MAX_WORKERS = 64`).

## Serialization

Records are serialized with `bincode`. Each record begins with:

```rust
struct RecordHeader {
    magic: u32,      // sanity check: 0x5A504C44 ("ZPLD")
    version: u32,    // record format version
    length: u32,     // byte length of the serialized payload that follows
}
```

The `version` field drives migration. When a binary reads a record with a
lower version than it expects, it runs the upgrade path. Records with a
higher version than expected are rejected with an error.

## Concurrency

The global lock in the Header is a `pthread_mutex_t` initialized with
`PTHREAD_PROCESS_SHARED`. Any process reading or writing the StateStore
must hold this lock. The framework acquires and releases it automatically
around StateStore operations.

Workers have their own per-slot lock for fine-grained access during handoff.

## Migration Example

When you add a field to `WorkerState` in a new binary:

```rust
#[derive(Serialize, Deserialize)]
struct WorkerStateV1 {
    version: u32,
    packet_count: u64,
}

#[derive(Serialize, Deserialize)]
struct WorkerStateV2 {
    version: u32,
    packet_count: u64,
    byte_count: u64,   // new field
}

fn read_worker_state(raw: &[u8]) -> Result<WorkerStateV2> {
    let header: RecordHeader = bincode::deserialize(&raw[..12])?;
    match header.version {
        1 => {
            let v1: WorkerStateV1 = bincode::deserialize(&raw[12..])?;
            Ok(WorkerStateV2 {
                version: 2,
                packet_count: v1.packet_count,
                byte_count: 0,   // default for new field
            })
        }
        2 => Ok(bincode::deserialize(&raw[12..])?),
        v => Err(Error::UnknownStateVersion(v)),
    }
}
```

Always handle every version your binary might encounter. Never silently
ignore unknown versions — return an error and let the supervisor decide
whether to abort the patch.

## Lifecycle

The StateStore file is created by the supervisor on first startup. If the
file already exists (e.g. after a supervisor crash), the supervisor reads
it and reconnects to any workers whose PIDs are still alive.

The StateStore is not cleared on clean shutdown — it persists across
restarts within the same boot. It is cleared on reboot (it lives in `/run`).
This is intentional: XFRM state also does not survive reboot.
