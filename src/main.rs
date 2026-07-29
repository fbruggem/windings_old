use std::{
    collections::VecDeque,
    fmt::Debug,
    future::poll_fn,
    os::fd::{AsFd, OwnedFd},
    pin::{Pin, pin},
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
};

use async_io::Async;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, protocol::wl_registry};

#[derive(Debug)]
enum Event {
    Registry {
        name: u32,
        interface: String,
        version: u32,
    },
}

impl Dispatch<wl_registry::WlRegistry, ()> for Events {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Events>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.queue.push_back(Event::Registry {
                name,
                interface,
                version,
            });
        }
    }
}

#[derive(Debug)]
struct Events {
    queue: VecDeque<Event>,
}

impl Events {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

#[derive(Debug)]
struct App {
    conn: Connection,
    queue: EventQueue<Events>,
    events: Events,
    fd: Async<OwnedFd>,
}

impl App {
    pub fn new(conn: Connection, queue: EventQueue<Events>) -> Self {
        let fd = conn.as_fd().try_clone_to_owned().unwrap();
        let fd = Async::new(fd).unwrap();

        Self {
            conn,
            queue,
            events: Events::new(),
            fd,
        }
    }

    pub fn registry(&mut self) {
        let display = self.conn.display();
        let _registry = display.get_registry(&self.queue.handle(), ());
        let _ = self.queue.flush();
    }

    pub async fn next(&mut self) -> Event {
        self.await
    }
}

impl Future for App {
    type Output = Event;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let Self {
            conn,
            queue,
            events,
            fd,
        } = self.get_mut();
        loop {
            while let Poll::Ready(_) = fd.poll_readable(cx) {
                let _ = queue.prepare_read().unwrap().read();
            }

            let _ = queue.flush();

            let _ = queue.poll_dispatch_pending(cx, events);

            if let Some(ev) = events.queue.pop_back() {
                return Poll::Ready(ev);
            }

            if let Poll::Pending = fd.poll_readable(cx) {
                return Poll::Pending;
            }
        }
    }
}

// The main function of our program
#[tokio::main]
async fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let event_queue = conn.new_event_queue();

    let mut data = App::new(conn, event_queue);

    data.registry();

    loop {
        println!("{:?}", data.next().await);
    }

    // To actually receive the events, we invoke the `roundtrip` method. This method
    // is special and you will generally only invoke it during the setup of your program:
    // it will block until the server has received and processed all the messages you've
    // sent up to now.
    //
    // In our case, that means it'll block until the server has received our
    // wl_display.get_registry request, and as a reaction has sent us a batch of
    // wl_registry.global events.
    //

    // `roundtrip` will then empty the internal buffer of the queue it has been invoked
    // on, and thus invoke our `Dispatch` implementation that prints the list of advertised
    // globals.
    // let mut app = AppData::new();
    // let receiver = app.receiver.clone();
    // event_queue.roundtrip(&mut app).unwrap();

    // poll_fn(|cx| receiver.try_recv)
}
struct ThreadWaker(thread::Thread);

impl ThreadWaker {
    fn new(thread: thread::Thread) -> Self {
        Self(thread)
    }
}

impl Wake for ThreadWaker {
    fn wake(self: std::sync::Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(f: F) -> F::Output
where
    F::Output: Debug,
{
    let mut f = Box::pin(f);
    let waker = Waker::from(Arc::new(ThreadWaker::new(thread::current())));
    let mut cx = Context::from_waker(&waker);

    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Pending => thread::park(),
            Poll::Ready(v) => {
                println!("{:?}", v)
            }
        }
    }
}
