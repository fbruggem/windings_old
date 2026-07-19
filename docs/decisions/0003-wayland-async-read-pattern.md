# ADR-003: Wayland Async Read Pattern via prepare_read() + async_io

**Date:** 2026-07-11
**Status:** Accepted

## Context

`wayland-client`'s `EventQueue::poll_dispatch_pending()` alone is not sufficient for
non-blocking async use. Calling it without first reading data from the socket can block
or return stale state. The correct multi-step handshake is poorly documented; the
discovery cost was ~2 hours of debugging (see [wayland-rs #570][wrs570]).

The core problem: `poll_dispatch_pending` only dispatches events that are already
buffered in the queue. If the kernel socket buffer has new data that hasn't been read
yet, those events are invisible. You must explicitly read from the connection fd first.

## Decision

Wrap the connection fd in `async_io::Async` and drive read/dispatch/flush in a manual
`Future::poll` implementation.

### Setup (once, before entering the async loop)

```rust
let c_fd = connection
    .prepare_read()
    .unwrap()
    .connection_fd()
    .try_clone_to_owned()?;
let c_fd = Async::new(c_fd)?;
```

`prepare_read()` is called here only to borrow the fd for cloning — the read guard is
dropped immediately. The cloned fd is then handed to `async_io` for readiness polling.

### Poll implementation (called by the async runtime on every wakeup)

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let Self { queue, c_fd, state } = self.get_mut();

    state.waker = cx.waker().clone();

    // 1. Drain the socket into the wayland-client buffer.
    while let Poll::Ready(()) = c_fd.poll_readable(cx)? {
        match queue.prepare_read().unwrap().read() {
            Ok(_) => (),
            // WouldBlock is not an error — fd became readable but no full
            // message arrived yet (e.g. partial write from compositor).
            Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => (),
            Err(e) => Err(e)?,
        }
    }

    // 2. Dispatch all buffered events into the State handler.
    let _ = queue.poll_dispatch_pending(cx, state)?;

    // 3. Make progress on pending user futures.
    let _ = state.user_queue.poll_next_unpin(cx);

    // 4. Flush outgoing requests to the compositor.
    while let Poll::Ready(()) = c_fd.poll_writable(cx)? {
        match queue.flush() {
            Ok(()) => break,
            Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => (),
            Err(e) => Err(e)?,
        }
    }

    Poll::Pending
}
```

### Dispatch handler (where events reach user code)

```rust
impl<R, F> Dispatch<WlRegistry, ()> for State<R, F> {
    fn event(state: &mut Self, _: &WlRegistry, event: Event, ..) {
        let mut future = (state.runner)(event);
        // Poll once immediately; queue only if the future yields.
        if future.poll_unpin(&mut Context::from_waker(&state.waker)).is_pending() {
            state.user_queue.push(future);
        }
    }
}
```

### Full working prototype

See the code block daxpedda posted on 2026-07-11 for a compilable end-to-end example
(connects to the compositor, prints the global registry, exits cleanly).

## Considered Alternatives

| Approach | Why it doesn't work |
|----------|---------------------|
| `poll_dispatch_pending` alone | Only dispatches already-buffered events; misses data still in the kernel socket buffer |
| Blocking `dispatch()` in a thread | Defeats async; requires a dedicated OS thread and synchronization |
| `queue.blocking_dispatch()` | Blocks the executor thread; breaks Wayland timeout handling |

## Consequences

- This pattern is **not obvious from the wayland-client docs** — anyone arriving fresh
  will likely waste hours on the same path. This ADR exists precisely to short-circuit
  that.
- `WouldBlock` on read must be swallowed silently (it is normal, not an error).
- The waker stored in `State` must be kept up-to-date each poll; the dispatch handler
  uses it to poll user futures in the right async context.
- The fd clone must survive the lifetime of the event loop — do not drop it early.
- This has only been partially verified (global registry listing works; full window
  lifecycle needs more implementation before it can be tested end-to-end).

[wrs570]: https://github.com/Smithay/wayland-rs/issues/570
