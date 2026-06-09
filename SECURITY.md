# Security Policy

## Supported Versions

Only the latest release on `main` is actively supported with security fixes.

## Reporting a Vulnerability

Do **not** open a public GitHub issue for security vulnerabilities.

To report a vulnerability, email the maintainers directly. Include:

- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept
- Affected versions
- Any suggested mitigations if you have them

You will receive an acknowledgement within 48 hours and a resolution timeline
within 7 days. We will credit you in the changelog unless you request otherwise.

## Scope

zpld processes network traffic and manages privileged kernel state (XFRM SAs,
routing entries). The following are in scope for security reports:

- Memory safety issues in `unsafe` blocks
- Privilege escalation via the supervisor control socket
- State store corruption allowing injection of malicious kernel state
- IPC protocol vulnerabilities between supervisor and workers
- Denial of service against the supervisor or a worker

## Out of Scope

- Vulnerabilities in third-party crates (report these upstream)
- Issues requiring physical access to the host
