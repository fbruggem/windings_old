use std::{
    future::poll_fn,
    os::fd::{AsFd, AsRawFd},
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender, sync_channel},
    },
};

use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, protocol::wl_registry};
// This struct represents the state of our app. This simple app does not
// need any state, but this type still supports the `Dispatch` implementations.
struct AppData {
    sender: SyncSender<Event>,
    receiver: Arc<Receiver<Event>>,
}

enum Event {
    Hehe,
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
            println!("INNER - [{}] {} (v{})", name, interface, version);
        }
    }
}

struct TEMP;

struct Data {
    conn: Connection,
    queue: EventQueue<TEMP>,
}

impl Data {
    pub fn new(conn: Connection, queue: EventQueue<TEMP>) -> Self {
        Self { conn, queue }
    }

    pub fn registry(&mut self) {
        let display = self.conn.display();
        let _registry = display.get_registry(&self.queue.handle(), ());
    }
    pub fn rount_trip(&mut self) {
        self.queue.roundtrip(&mut TEMP).unwrap();
    }
}

impl Future for Data {
    type Output = Event;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let fd = async_io::Async::new(self.conn.as_fd()).unwrap();
        // fd.poll_readable(cx).

        unimplemented!();
    }
}

// The main function of our program
fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let event_queue = conn.new_event_queue();

    let mut data = Data::new(conn, event_queue);

    data.registry();
    data.rount_trip();
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
// struct ThreadWaker(thread::Thread);
//
// impl ThreadWaker {
//     fn new(thread: thread::Thread) -> Self {
//         Self(thread)
//     }
// }
//
// impl Wake for ThreadWaker {
//     fn wake(self: std::sync::Arc<Self>) {
//         self.0.unpark();
//     }
// }
//
// fn block_on<F: Future>(f: F) -> F::Output {
//     let mut f = Box::pin(f);
//     let waker = Waker::from(Arc::new(ThreadWaker::new(thread::current())));
//     let mut cx = Context::from_waker(&waker);
//
//     loop {
//         match f.as_mut().poll(&mut cx) {
//             Poll::Pending => thread::park(),
//             Poll::Ready(v) => return v,
//         }
//     }
// }
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
