# Requirements

Each file in this directory is the requirements document for one development
phase. Requirements define **what** must be built and how we know it is done.
The design documents in `docs/` define **how** things work.

## Requirement IDs

Every functional requirement has a unique ID: `<PREFIX>-FR-<NN>`

| Prefix | Component |
|---|---|
| `FT` | Framework Types and Traits (Phase 1) |
| `SS` | StateStore (Phase 2) |
| `IPC` | IPC Wire Format (Phase 3) |
| `SUP` | Supervisor Core (Phase 4) |
| `UDP` | UDP Counter Worker (Phase 5) |
| `PS` | Patch Scheduler (Phase 6) |
| `CTL` | zpldctl CLI (Phase 7) |
| `SPT` | Supervisor Self-Patch (Phase 8) |

Non-functional requirements use `NFR` instead of `FR`: e.g. `FT-NFR-01`.

## How to Use These in Practice

- **Commits:** Reference the requirement ID in the commit message.
  `feat(framework): implement Worker trait [FT-FR-01]`
- **Tests:** Name tests after the requirement they verify.
  `fn test_ft_fr_01_worker_trait_init_called_on_startup()`
- **PRs:** List the requirement IDs being addressed in the PR description.
- **GitHub Issues:** One issue per requirement or logical group of requirements.

## Priority Levels

| Level | Meaning |
|---|---|
| `P0 — Must Have` | Blocking. Phase is incomplete without it. |
| `P1 — Should Have` | Important but not blocking phase completion. |
| `P2 — Nice to Have` | Non-blocking. Deferred to a future phase. |

## Status Values

`Not Started` → `In Progress` → `In Review` → `Done`
