use std::{
    collections::VecDeque,
    fmt::Debug,
    os::fd::{AsFd, OwnedFd},
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
};

use async_io::Async;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, protocol::wl_registry};

#[derive(Debug)]
enum Event {
    Registry(String),
}

// Implement `Dispatch<WlRegistry, ()> for our state. This provides the logic
// to be able to process events for the wl_registry interface.
//
// The second type parameter is the user-data of our implementation. It is a
// mechanism that allows you to associate a value to each particular Wayland
// object, and allow different dispatching logic depending on the type of the
// associated value.
//
// In this example, we just use () as we don't have any value to associate. See
// the `Dispatch` documentation for more details about this.
impl Dispatch<wl_registry::WlRegistry, ()> for Events {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Events>,
    ) {
        // When receiving events from the wl_registry, we are only interested in the
        // `global` event, which signals a new available global.
        // When receiving this event, we just print its characteristics in this example.
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.queue.push_back(Event::Registry(format!(
                "INNER - [{}] {} (v{})",
                name, interface, version
            )));
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

    pub fn push(&mut self, event: Event) {
        self.queue.push_back(event);
    }
    pub fn pop(&mut self) -> Option<Event> {
        self.queue.pop_front()
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
fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let event_queue = conn.new_event_queue();

    let mut data = App::new(conn, event_queue);

    data.registry();
    let a = block_on(data);

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
