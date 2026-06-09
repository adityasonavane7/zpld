# udp-counter — Reference zpld Worker

`udp-counter` is the reference implementation of a zpld worker. It does one
simple thing: listen on a UDP port, count packets arriving from a configured
source address, and survive a worker swap with the count intact.

Read this worker's source alongside [docs/worker-contract.md](../../docs/worker-contract.md).
Every decision in this implementation is documented to explain how the
worker contract maps to real code.

## What This Worker Tests

- Worker startup, ready signal, and heartbeat
- StateStore write (packet count survives a swap)
- File descriptor passing (UDP socket handed to new worker via SCM_RIGHTS)
- Supervisor lazy update lifecycle end-to-end
- StateStore version migration (intentionally tested by changing the struct)

## Configuration

See `config/example.toml` for all options.

```toml
[worker]
listen_addr = "0.0.0.0:9000"
source_filter = "192.168.1.10"   # only count packets from this address
heartbeat_interval_secs = 5
```

## Running with Docker

```bash
cd docker
docker compose up
```

This starts:
- `supervisor` — zpld-supervisor managing one udp-counter worker
- `sender` — a container that sends UDP packets every second

Watch the count in `zpldctl status udp-counter-1`. Then trigger a swap:

```bash
docker exec supervisor zpldctl update worker udp-counter-1
```

Verify the count continues from where it left off without resetting.

## Testing State Migration

1. Modify `WorkerState` in `src/main.rs` — add a new field with a default
2. Bump the version constant
3. Implement the migration arm in `read_worker_state()`
4. `cargo build`
5. Trigger a worker swap — the new binary should read the old state and migrate

## zpldctl Output

```
$ zpldctl status udp-counter-1
ID:      udp-counter-1
Binary:  /usr/sbin/udp-counter  v0.1.0
State:   Running
Health:  OK
Uptime:  1h 23m

[Worker Status]
listen_addr:    0.0.0.0:9000
source_filter:  192.168.1.10
packet_count:   42,817
byte_count:     4,710,048
```
