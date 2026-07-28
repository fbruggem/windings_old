use std::{
    collections::VecDeque,
    fmt::Debug,
    future::{Ready, poll_fn},
    os::fd::{AsFd, BorrowedFd, OwnedFd},
    sync::{
        Arc,
        mpsc::{Receiver, Sender, SyncSender, channel, sync_channel},
    },
    task::{Context, Poll, Wake, Waker},
    thread,
};

use async_io::Async;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, protocol::wl_registry};
// This struct represents the state of our app. This simple app does not
// need any state, but this type still supports the `Dispatch` implementations.
struct AppData {
    sender: SyncSender<Event>,
    receiver: Arc<Receiver<Event>>,
}

#[derive(Debug)]
enum Event {
    Registry(String),
}

const SYNC_SIZE: usize = 100;

impl AppData {
    pub fn new() -> Self {
        let (sender, receiver) = sync_channel(SYNC_SIZE);
        Self {
            sender,
            receiver: Arc::new(receiver),
        }
    }
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
impl Dispatch<wl_registry::WlRegistry, ()> for TEMP {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<TEMP>,
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
            println!("yess");
        }
    }
}

#[derive(Debug)]
struct TEMP {
    queue: VecDeque<Event>,
}

impl TEMP {
    pub fn new() -> Self {
        TEMP {
            queue: VecDeque::new(),
        }
    }
}

impl TEMP {}

#[derive(Debug)]
struct Data {
    conn: Connection,
    queue: EventQueue<TEMP>,
    temp: TEMP,
    fd: Async<OwnedFd>,
}

impl Data {
    pub fn new(conn: Connection, queue: EventQueue<TEMP>) -> Self {
        let fd = conn.as_fd().try_clone_to_owned().unwrap();
        let fd = Async::new(fd).unwrap();

        Self {
            conn,
            queue,
            temp: TEMP::new(),
            fd,
        }
    }

    pub fn registry(&mut self) {
        let display = self.conn.display();
        let _registry = display.get_registry(&self.queue.handle(), ());
        let _ = self.queue.flush();
    }
}

impl Future for Data {
    type Output = Event;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let Self {
            conn,
            queue,
            temp,
            fd,
        } = self.get_mut();
        loop {
            while let Poll::Ready(_) = fd.poll_readable(cx) {
                let _ = queue.prepare_read().unwrap().read();
            }

            let _ = queue.poll_dispatch_pending(cx, temp);

            if let Some(ev) = temp.queue.pop_back() {
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

    let mut data = Data::new(conn, event_queue);

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

// use std::{
//     io,
//     sync::{Arc, mpsc::channel},
//     task::{Context, Poll, Wake, Waker},
//     thread,
// };
//
// use wayland_client::{
//     Connection, Dispatch, QueueHandle,
//     protocol::{wl_display::WlDisplay, wl_registry},
// };
// // This struct represents the state of our app. This simple app does not
// // need any state, but this type still supports the `Dispatch` implementations.
// struct AppData;
//
//
// // Implement `Dispatch<WlRegistry, ()> for our state. This provides the logic
// // to be able to process events for the wl_registry interface.
// //
// // The second type parameter is the user-data of our implementation. It is a
// // mechanism that allows you to associate a value to each particular Wayland
// // object, and allow different dispatching logic depending on the type of the
// // associated value.
// //
// // In this example, we just use () as we don't have any value to associate. See
// // the `Dispatch` documentation for more details about this.
// impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
//     fn event(
//         _state: &mut Self,
//         _: &wl_registry::WlRegistry,
//         event: wl_registry::Event,
//         _: &(),
//         _: &Connection,
//         _: &QueueHandle<AppData>,
//     ) {
//         // When receiving events from the wl_registry, we are only interested in the
//         // `global` event, which signals a new available global.
//         // When receiving this event, we just print its characteristics in this example.
//         if let wl_registry::Event::Global {
//             name,
//             interface,
//             version,
//         } = event
//         {
//             println!("[{}] {} (v{})", name, interface, version);
//         }
//     }
// }
//
// #[derive(Debug)]
// enum Event {
//     One,
// }
//
// struct Wayland;
//
// impl Future for Wayland {
//     type Output = Event;
//     fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         Poll::Pending
//     }
// }
//
// // The main function of our program
// fn main() {
//     let (s, r) = channel();
//
//     s.send(3);
//     s.
//     // Create a Wayland connection by connecting to the server through the
//     // environment-provided configuration.
//     let conn = Connection::connect_to_env().unwrap();
//
//     // Retrieve the WlDisplay Wayland object from the connection. This object is
//     // the starting point of any Wayland program, from which all other objects will
//     // be created.
//     let display = conn.display();
//
//     // Create an event queue for our event processing
//     let mut event_queue = conn.new_event_queue();
//     // And get its handle to associate new objects to it
//     let qh = event_queue.handle();
//
//     println!("{:?}", block_on(Wayland));
//
//     // Create a wl_registry object by sending the wl_display.get_registry request.
//     // This method takes two arguments: a handle to the queue that the newly created
//     // wl_registry will be assigned to, and the user-data that should be associated
//     // with this registry (here it is () as we don't need user-data).
//     let _registry = display.get_registry(&qh, ());
//
//     // At this point everything is ready, and we just need to wait to receive the events
//     // from the wl_registry. Our callback will print the advertised globals.
//     println!("Advertised globals:");
//
//     // To actually receive the events, we invoke the `roundtrip` method. This method
//     // is special and you will generally only invoke it during the setup of your program:
//     // it will block until the server has received and processed all the messages you've
//     // sent up to now.
//     //
//     // In our case, that means it'll block until the server has received our
//     // wl_display.get_registry request, and as a reaction has sent us a batch of
//     // wl_registry.global events.
//     //
//     // `roundtrip` will then empty the internal buffer of the queue it has been invoked
//     // on, and thus invoke our `Dispatch` implementation that prints the list of advertised
//     // globals.
//     event_queue.roundtrip(&mut AppData).unwrap();
// }
