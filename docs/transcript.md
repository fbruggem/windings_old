# Discord Chat Transcript

**Participants:** fbruggem (Felix), daxpedda
**Topic:** Winit rewrite — Wayland async event loop
**Dates:** 2026-07-08 to 2026-07-11

---

## 2026-07-08

**daxpedda** — Ha. Good one.
How is it going? Any progress learning Wayland?

**fbruggem** — nicht so viel - gestern war vesta krank und da haben wir unser "wochenede" gemacht weil sie am sa/so oft arbeitet - aber jetzt setzt ich mal dahinter

**daxpedda** — Nice!

---

**fbruggem** — meint einfach nur das `env!` here oder? Hab in den source code reingekuckt und die benutzen `env::var()` - wollt wissen ob das `!` was spezielles ist.

**daxpedda** — Hab mich auch ein bisschen versucht zu informieren über Wayland und diese ganze Server G'schicht.
Anscheinend benutzt Winit auch smithay, high-level Wrapper über wayland-*. Das sollte das Ganze etwas einfacher machen.

`<ident>!()` heißt Makro in Rust, that's all.
`env!` != `env::var()`. Auch wenn die Funktionalität sehr ähnlich sind, sie sind zwei verschiedene Symbole.

- `env!` is compile-time
- `env::var()` is run-time

Also Funktionalität nicht mal annäherend dasselbe, sry^^

**fbruggem** — was meinst mit ident? einfach ein placeholder als `println!()` is eine variante des `<ident>!()`?

**daxpedda** — Ein "Ident" in Programmiersprachen-Lingo heißt einfach nur ein valider Name eines Symbols.
E.g. `my_var` is ein Ident, `42_vienna` nicht, weil Symbole nicht mit Integers anfangen dürfen in Rust.

---

**fbruggem** — ok weil ich haeng grade bei `wayland_client` in der docu drinnen

**daxpedda** — Ich kann garnicht so viel dazu sagen. Soweit ich sehen kann is `smithay-client-toolkit` ein high-level wrapper über `wayland-client` also nehm ich an es is leichter zu benutzen.
Hab grad auch nachgeschaut — Winit verwendet doch `wayland-client` direkt. So kA.
Kannst immer bei Winit nachschauen wie sies gemacht haben.
Aber ja, wird natürlich nicht leicht so einen ganzen Compositor zu lernen.
(kannst auch in smithay-client-toolkit schauen zu was der high-level wrapper runterbricht)
Oder zuerst `smithay-client-toolkit` verwenden und später auf `wayland-client` runter.

**fbruggem** — safe - weil in winit habe ich eben die dependency `wayland-client` gefunden - und wenn die nur durch das smithay angesprochen worden wuerde wuerde dass ja nicht in der `cargo.toml` landen

**daxpedda** — Doch kannst schon auf beide dependen, aber tuts ja nicht in dem Fall 😉 Wär auch komisch.

> **Decision captured:** [ADR-001 — Use wayland-client directly](decisions/0001-wayland-client-library-choice.md)

---

## 2026-07-09

**fbruggem** — just saw

```rust
struct App;
func(&mut App);
```

for the first time and i know why it makes sense - you instantiate it on call and pass it mutably. But idk it looks cursed as fuck heh

**daxpedda** — It does, lol.

---

**fbruggem** — man - i get flashbacks to tokio right now when i see all this dispatch handlers heh

**daxpedda** — (context on what he's working on: translating minicov to Rust for wasm-bindgen, finishing a consulting report, working on js-bindgen, a game with his brother, discussing Android backend for Winit, writing a book)

---

**daxpedda** — Okay.
Winit hat auch `pump_app_events()`. Es kann sein das Wayland besser dafür ausgelegt is. Wir könnten dann die blocking `run_app()` for Wayland über das pumpen implementieren. We will see.

Wir sollten mal syncen wieder, hab ein bissi was zu berichten von meiner Recherche und wir sollten auch schauen wie overwhelmed du bist ❤️

**fbruggem** — My impostersyndrome every 5 mins "why do you not understand it bro - he will think u are stupid be faster - idiot"
But dealing so far with it.
I always say "thx for the input you can be on the Beifahrer sitz and shut the fuck up"

**daxpedda** — Hahaha, are you insane?^^
There is a reason why nobody is maintaining Winit: nobody can!
This takes a long time to properly learn and understand.
The fact that you are attempting it is more than what most ppl ever did.
Seriously, there's only like 20 people in the Rust open-source community who even know what this is.

**fbruggem** — Like wayland? Winit or what u mean?
Also i think its ok to just read for like 3/4 days

**daxpedda** — Wayland. Yes! At an absolute minimum!

**fbruggem** — Like only 20 people in the rust system know how the interface to wayland works? That sounds too little.
I mean understand it deep enough to contribute to Winit. Understanding the interface isn't enough for that.

**daxpedda** — Other backends there is far fewer to none. Android its like 3 people.

> **[Call — 1 hour — 2026-07-09, 12:26 PM]**

---

## 2026-07-10

**fbruggem** — nur dass ich das gestern richtig verstanden habe

Wir wollen eine funktion zb

```rust
async fn open_window(...) {
    // returns when the window has opened or when an error occured?
}
```

**daxpedda** — Yes.
Ich weiß garnicht ob das geht mit Wayland, weil es gibt eine timeout Periode wenn du nicht dem Server antwortest.
Also du musst herausfinden was passiert wenn du das Fenster öffnest, und dann für eine lange Periode nicht mehr interagierst.

**fbruggem** — ja wenn du kein pong sendest is der server so - hehe was bruda - und killed die connection.
Also es geht mit `wayland_client`, dass man sich eine `.await` variante baut mit der man auf incoming stuff warten kann.
Jetzt gibts zwei Möglichkeiten:
- du hast einen thread der permanent schaut — hey is there something new — yes? dispatch events
- oder du machst dass wenn du `open_window()` callst, darin diese loop machst, weil `dispatch_events()` ja alle events dispatched.

**daxpedda** — Okay, also wir brauchen Antworten auf folgende Fragen:
- Wie lang ist das Timeout, ca?
- Was heißt das wenn die Connection gekilled wird? Werden einfach folgende requests failen oder gibts da irgendwo unsafe?
- Was passiert mit dem Drawing Surface?

Das is was wir machen sollten aber async. Warte ich sketche kurz.

**fbruggem** — soweit ichs bis jetzt weis is es implementation defined wann pings gesendet werden und was das timeout ist — maybe its hidden in a RFC.
I GUESS die "application is not responding" message.
Also in der function hast eine `wait_for_next_socket_read().await` — heist es ist kein busy loop — heist der user/winit waere verantwortlich das zu pumpen.
Bzw in jeder funktion hast du diese funktion - somit wartet "jedes event" in "jedem wait" weil dispatch und so.

---

**daxpedda** — Sketch des async event loop designs:

```rust
fn run_event_loop<F: Future>(executor: impl Fn(F), f: impl Fn(Event) -> Future) {
    executor(async move {
        let mut user_queue = FuturesUnordered::new();
        // some way the user can exit the loop
        loop {
            select! {
                event = poll_your_thing => poll_fn(|cx| {
                    let mut future = f(event);
                    // We poll only a single time before queuing the user future.
                    if let Poll::Pending = future.poll(cx) {
                        user_queue.push(future);
                    }
                    Poll::Ready(())
                }).await,
                _ = user_queue => (),
            };
        }
    });
}

// E.g. Tokio Runtime.
let runtime = Runtime::new().unwrap();
// We pass the executor and work it ourselves.
// This way we control when we are polling instead of the user.
run_event_loop(|f| runtime.block_on(f), |event| async { ... });
```

Macht das iwie Sinn?

Ohne den Executor:

```rust
async fn open_window(f: impl Fn(Event) -> Future) {
    let mut user_queue = FuturesUnordered::new();
    loop {
        select! {
            event = poll_your_thing => poll_fn(|cx| {
                let mut future = f(event);
                if let Poll::Pending = future.poll(cx) {
                    user_queue.push(future);
                }
                Poll::Ready(())
            }).await,
            _ = user_queue => (),
        };
    }
}

open_window(|event| async { ... }).await;

// The user could just poll once and forget:
let mut event_loop = open_window(|event| async { ... });
poll_fn(|cx| event_loop.poll(cx)).await;
// ...
// Now the runtime would time out.
```

ok ok! also der user sollte nicht sich damit auseinander setzen wann er pollen will und wann er pumpen will — OK!

**daxpedda** — Yes. Wir können später ein Pumpen Interface on popular demand exposen.
Fürs erste is das einfach ein Blödsinn weil es kann ja einfach Timeout verursachen.

**fbruggem** — ok sure — und dass ist das interne open_window nicht das exposed, weil der user ja nicht den executor hat?

**daxpedda** — Doch, der User hat den Executor.
Ganz unten führ ichs ja mit einem Tokio Executor aus.
Später dann sollte die Funktion nicht `open_window()` sein sondern `run_event_loop()` und anstatt einer Funktion passt er eine `ApplicationHandler` Implementation. Aber die Signatur wird ca. so aussehn.

> **Decision captured:** [ADR-002 — Async event loop design](decisions/0002-async-event-loop-design.md)

---

## 2026-07-11

**fbruggem** — ok also - ich checke dass wir ne function rein packen die das future ausfuert hier.
Und die user_queue ist dafür da wenn wir mehrere sachen gleichzeitig machen wollen i guess.

Und bei `event = poll ... poll_fn(|cx|` — muss dass doch `Poll` returnen oder?
Und `f` hier waere dann was gemacht wird wenn das window geöffnet wurde oder gefailed ist oder?
Also where do you get `cx` from — like the context?
Why do we poll once?

**daxpedda** — [`std::future::poll_fn()`](https://doc.rust-lang.org/std/future/fn.poll_fn.html) — Das lässt dich einfach nur in einem async context 1x pollen; `cx` wird dir von `poll_fn` dafür gegeben.
Wir können ja nicht einfach die user Funktion ausführen weil es is async.
Wir wollen es aber zumindest 1x pollen damit es ausgeführt wird wenn das event arrived. Dafür is ja das ganze da.

**fbruggem** — ahhh - du hast `c` und `cx` geschrieben, deswegen war ich verwirrt.

**daxpedda** — Aso, sorry. Hab das nur so in Discord reingeschrieben^^ — Verbessert.

---

**fbruggem** — ok und wiso wollen wir es mindestens einmal runnen — wäre dann nicht eine sync callback closure besser?
Weil sonst wird ja dem user kommuniziert du kannst hier async arbeiten.
Bzw wir packen es dann in die user_queue rein dass es mehrmals gepolled wird ne — also einmal anstarten und dann in die queue die es immer wieder called?

**daxpedda** — Nein, du tust es ja nur in die Queue rein wenns yielded. Sonst nicht.
Du willst es mindestens 1x runnen damit genau das passiert was der user erwartet: seine Funktion wird ausgeführt wenn das Event arrived.
Wenn dus nicht zumindest 1x pollst:
1. Das Event arrived.
2. User Funktion wird gequeued.
3. Wir pollen die Queue oder unseren Event Receiver.
4. Etwas passiert bevor die User Funktion ausgeführt wird obwohl das Event angekommen is.

Du willst auch nicht mehr als 1x pollen weil wir verlassen uns darauf das Future richtig funktioniert. Das heißt wenn die Future `Poll::Pending` returned dann heißt das es nicht sofort wieder ready — async halt.

**fbruggem** — ja also wenns pending gibt packen wirs in die queue aber wofuer brauchen wir die queue, können wir nicht einfach:

```rust
let event = poll_your_thing().await;
user_function(event).await;
```

weil es wird ja immer nur die eine future drinnen sein nicht?

**daxpedda** — Was macht die erste Zeile in deinem Beispiel hier?
Also wenn du `.await` macht dann wird halt nix mehr weitergehen. Das geht ja auch nicht.
Wir wollen ja Arbeit machen während die User Funktion nichts tut.
Wir wollen nur `.await` machen während wir auf alles warten, nicht nur auf eine Sache.
Wenn du einfach schreibst `user_function(event).await` dann wartet alles andere darauf das die User Funktion fertig wird.
Deswegen haben wir ja `select!`.

> **[Call — 2 hours — 2026-07-11, 14:28]**

---

**daxpedda** — Hm, also ich habs eigentlich schon lange her hingekriegt.
Ich weiß einfach nur nicht wie Wayland funktioniert.
Anscheinend muss man ein bisschen mehr machen um `poll_dispatch_pending()` zum laufen zu bringen.
Iwas mit `prepare_read()`, but no idea.
Aha: https://github.com/Smithay/wayland-rs/issues/570.
Sicher zwei Stunden verschwendet daran. Das war dumm.

**daxpedda** — Okay, ich probier jetzt nochmal mit dem `prepare_read()`.
Ja habs ... die Doku is einfach nur im Arsch. Schade Schokolade.
Ich kanns nicht ganz verifien alles weil dafür brauchma ein bissi mehr als nichtmal ein Fenster, aber den Rest kriegst du schon hin denke ich.

Working prototype:

```rust
use std::future;
use std::io::ErrorKind;
use std::os::unix::prelude::OwnedFd;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use anyhow::Result;
use async_io::Async;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt, executor};
use wayland_client::backend::WaylandError;
use wayland_client::protocol::wl_registry::{Event, WlRegistry};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};

fn main() -> Result<()> {
    EventLoop::run(executor::block_on, |event| {
        if let Event::Global { name, interface, version } = event {
            println!("[{}] {} (v{})", name, interface, version);
        }
        future::ready(())
    })
}

struct EventLoop<R: Fn(Event) -> F + Unpin, F: Future<Output = ()> + Unpin> {
    queue: EventQueue<State<R, F>>,
    c_fd: Async<OwnedFd>,
    state: State<R, F>,
}

struct State<R: Fn(Event) -> F + Unpin, F: Future<Output = ()> + Unpin> {
    runner: R,
    waker: Waker,
    user_queue: FuturesUnordered<F>,
}

impl<R: Fn(Event) -> F + Unpin + 'static, F: Future<Output = ()> + Unpin + 'static>
    EventLoop<R, F>
{
    #[allow(clippy::await_holding_refcell_ref)]
    fn run(executor: impl Fn(EventLoop<R, F>) -> Result<()>, runner: R) -> Result<()> {
        let connection = Connection::connect_to_env().unwrap();
        let event_queue = connection.new_event_queue();
        let handle = event_queue.handle();
        let display = connection.display();
        display.get_registry(&handle, ());

        let c_fd = connection
            .prepare_read()
            .unwrap()
            .connection_fd()
            .try_clone_to_owned()?;
        let c_fd = Async::new(c_fd)?;

        let event_loop = EventLoop {
            queue: event_queue,
            c_fd,
            state: State {
                runner,
                waker: Waker::noop().clone(),
                user_queue: FuturesUnordered::new(),
            },
        };

        executor(event_loop)
    }
}

impl<R: Fn(Event) -> F + Unpin, F: Future<Output = ()> + Unpin> Future for EventLoop<R, F> {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { queue, c_fd, state } = self.get_mut();

        state.waker = cx.waker().clone();

        while let Poll::Ready(()) = c_fd.poll_readable(cx)? {
            match queue.prepare_read().unwrap().read() {
                Ok(_) => (),
                Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => (),
                Err(error) => Err(error)?,
            }
        }

        let _ = queue.poll_dispatch_pending(cx, state)?;
        let _ = state.user_queue.poll_next_unpin(cx);

        while let Poll::Ready(()) = c_fd.poll_writable(cx)? {
            match queue.flush() {
                Ok(()) => break,
                Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => (),
                Err(e) => Err(e)?,
            }
        }

        Poll::Pending
    }
}

impl<R: Fn(Event) -> F + Unpin, F: Future<Output = ()> + Unpin> Dispatch<WlRegistry, ()>
    for State<R, F>
{
    fn event(
        state: &mut Self,
        _: &WlRegistry,
        event: Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<State<R, F>>,
    ) {
        let Event::Global { name, interface, version } = event else {
            return;
        };

        let mut future = (state.runner)(Event::Global { name, interface, version });

        if future
            .poll_unpin(&mut Context::from_waker(&state.waker))
            .is_pending()
        {
            state.user_queue.push(future);
        }
    }
}
```

> **Decision captured:** [ADR-003 — Wayland async read pattern](decisions/0003-wayland-async-read-pattern.md)
