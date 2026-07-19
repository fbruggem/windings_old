# ADR-001: Use wayland-client Directly, Not smithay-client-toolkit

**Date:** 2026-07-08
**Status:** Accepted

## Context

When implementing Wayland support we had two realistic library options:

- **`wayland-client`** — the low-level Wayland protocol binding. Requires writing protocol
  dispatch code by hand; gives full control over every interaction with the compositor.
- **`smithay-client-toolkit`** — a high-level wrapper over `wayland-client` that
  auto-generates binding boilerplate via macros (visible as code-gen output in the
  smithay screenshot shared in chat).

Key observation: upstream winit uses `wayland-client` directly. If winit depended on
smithay-client-toolkit it would still pull in `wayland-client` transitively, but that
is not the case — `wayland-client` appears as a direct dependency in winit's `Cargo.toml`.

## Decision

Use **`wayland-client` directly**.

## Considered Options

| Option | Pro | Con |
|--------|-----|-----|
| `wayland-client` | Full control; matches winit upstream; no macro magic | More boilerplate; steeper learning curve |
| `smithay-client-toolkit` | Less boilerplate; higher-level abstractions | Extra abstraction layer; diverges from how winit does it |

Both can be depended on simultaneously, but doing so without a clear reason would be
confusing.

## Consequences

- We write more protocol glue code by hand.
- We can reference `smithay-client-toolkit` source to understand what it abstracts away
  whenever we get stuck.
- Staying aligned with winit's own dependency graph makes it easier to eventually
  upstream changes.
- The Wayland expertise required here is non-trivial; roughly ~20 people in the broader
  Rust open-source community understand it deeply enough to contribute at this level —
  so the learning curve is expected and normal.
