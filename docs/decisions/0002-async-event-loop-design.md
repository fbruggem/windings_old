# ADR-002: Async Event Loop Design with External Executor

**Date:** 2026-07-10
**Status:** Accepted

## Context

We want an async-first API for the event loop. The key constraints are:

1. **Wayland server-side timeout.** The compositor will kill the connection if the client
   stops responding. Exactly how long this takes and what triggers it is
   implementation-defined (possibly in a Wayland RFC), but the "application not
   responding" dialog is the visible symptom.
2. **Concurrent user futures.** A user callback handling one event should not block
   delivery of the next event — the client must keep pumping the Wayland socket.
3. **Executor choice belongs to the caller.** We should not hard-code a runtime (e.g.
   Tokio); the caller passes their executor in.

Winit already has `pump_app_events()`, which lets callers control when the event queue
is drained. That is a tempting model but conflicts with constraint (1) above.

## Decision

The public entry point will be:

```rust
fn run_event_loop<F: Future>(
    executor: impl Fn(F),
    handler: impl Fn(Event) -> F,
)
```

The executor is provided by the caller (e.g. `|f| runtime.block_on(f)`). Winit drives
the internal future; the caller never touches the poll cycle directly.

Internally the event loop is a `Future` that:

1. Polls the Wayland socket for new events via `select!`.
2. On each arrived event: calls `handler(event)`, polls the returned future **once**.
   - If `Poll::Ready` — done, nothing to queue.
   - If `Poll::Pending` — pushes the future into a `FuturesUnordered` user queue.
3. Also polls the user queue in the same `select!` arm so pending user work makes
   progress concurrently.

```rust
fn run_event_loop<F: Future>(executor: impl Fn(F), f: impl Fn(Event) -> F) {
    executor(async move {
        let mut user_queue = FuturesUnordered::new();
        loop {
            select! {
                event = poll_wayland_event => poll_fn(|cx| {
                    let mut future = f(event);
                    if let Poll::Pending = future.poll(cx) {
                        user_queue.push(future);
                    }
                    Poll::Ready(())
                }).await,
                _ = user_queue.next() => (),
            };
        }
    });
}
```

The user-facing name will eventually be `run_event_loop()` (not `open_window()`), and
the handler signature will likely become an `ApplicationHandler` trait impl rather than
a bare closure, but the polling model stays the same.

## Considered Options

### Option A: Expose pump interface (like current winit)

Allow users to call `pump_app_events()` themselves.

**Rejected because:** if the caller decides not to pump (or delays pumping), the Wayland
server can time out and kill the connection. The failure mode is silent and hard to debug.
Can be added later on popular demand once we understand the safe envelope.

### Option B: Simple sequential await

```rust
async fn run_event_loop(handler: impl Fn(Event) -> impl Future) {
    loop {
        let event = poll_wayland_event().await;
        handler(event).await; // blocks everything else
    }
}
```

**Rejected because:** `.await`-ing the handler serialises everything. While the handler
is running, no new events are processed and no flushing happens — the Wayland connection
starves.

### Option C (chosen): select! + FuturesUnordered

See decision above.

## Consequences

- Users do not need to reason about polling cadence or Wayland timeouts.
- Wayland keepalive is managed internally; the connection stays healthy as long as our
  poll loop runs.
- The pump interface is intentionally absent for now; it is a footgun on Wayland. We
  can expose it later behind a feature flag or platform-specific API if demand arises.
- Callers must supply an async executor. This is a minor ergonomic cost but gives them
  full control over runtime choice (Tokio, async-std, smol, …).
- Polling the user future exactly once before queuing relies on the contract that
  `Poll::Pending` means "not immediately ready again" — i.e. the future will wake the
  waker when it makes progress. This is standard async Rust semantics.
