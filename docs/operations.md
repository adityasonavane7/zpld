# Operations

## Installation

### From Source

```bash
cargo build --release
install -m 755 target/release/zpld-supervisor /usr/sbin/
install -m 755 target/release/zpldctl /usr/bin/
install -m 755 target/release/udp-counter /usr/sbin/  # or your worker binary
```

### Configuration

```bash
mkdir -p /etc/zpld /run/zpld
install -m 640 crates/zpld-supervisor/config/example.toml /etc/zpld/supervisor.toml
# Edit /etc/zpld/supervisor.toml to configure your workers
```

### systemd

```bash
install -m 644 packaging/systemd/zpld-supervisor.service /etc/systemd/system/
install -m 644 packaging/systemd/zpld-supervisor.socket  /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now zpld-supervisor.socket
```

## Verifying the Setup

```bash
zpldctl list           # should show configured workers
zpldctl status <id>    # detailed status for one worker
```

## Applying a Patch

1. Build the new binary:
   ```bash
   cargo build --release --bin <worker-name>
   install -m 755 target/release/<worker-name> /usr/sbin/<worker-name>.new
   ```

2. Trigger a lazy update:
   ```bash
   zpldctl update worker <id>
   ```

3. Monitor progress:
   ```bash
   zpldctl events
   ```

The supervisor will wait for the worker to report stable, then swap it.
Traffic continues flowing through existing kernel state during the swap.

## Patching the Supervisor

```bash
cargo build --release --bin zpld-supervisor
install -m 755 target/release/zpld-supervisor /usr/sbin/zpld-supervisor.new
zpldctl update supervisor
```

The supervisor forks, execs the new binary, waits for it to signal ready,
then exits. Workers are unaffected.

## Monitoring

zpld logs via `tracing` to stdout/stderr, captured by systemd journal.

```bash
journalctl -u zpld-supervisor -f          # follow supervisor logs
journalctl -u zpld-supervisor --since -1h # last hour
zpldctl events                            # structured event stream
```

Key events to alert on:

| Event | Severity | Meaning |
|---|---|---|
| `worker.dead` | Critical | Worker died unexpectedly |
| `update.failed` | Error | Lazy update could not complete |
| `heartbeat.missed` | Warning | Worker missed heartbeat deadline |
| `stability.blocked` | Info | Update waiting on stability condition |

## Runtime Directories

| Path | Purpose |
|---|---|
| `/run/zpld/state.db` | StateStore (mmap'd, created at startup) |
| `/run/zpld/supervisor.sock` | Control socket for zpldctl |
| `/run/zpld/worker-<id>.sock` | Per-worker IPC socket |
| `/etc/zpld/supervisor.toml` | Supervisor configuration |
| `/etc/zpld/<worker>.toml` | Worker-specific configuration |

## Capabilities Required

The supervisor and workers require:

- `CAP_NET_ADMIN` — to manage kernel XFRM state, routing entries
- `CAP_NET_RAW` — for raw socket access (some workers)

The systemd unit file configures these via `AmbientCapabilities`. Running
as root is the simplest deployment; a dedicated `zpld` user with the above
capabilities is preferred for production.
