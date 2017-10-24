use futures::unsync::mpsc::{unbounded, UnboundedReceiver};
use futures::{Poll, Stream};
use stdweb;
use std::time::Duration;
use stdweb::unstable::TryInto;
use std::mem::ManuallyDrop;

pub struct IntervalStream {
    receiver: ManuallyDrop<UnboundedReceiver<()>>,
    interval_ref: f64,
}

impl Drop for IntervalStream {
    fn drop(&mut self) {
        unsafe {
            // Drop the Receiver before Sender. The Sender would otherwise try
            // to resubmit the Task to the Executor, which at least at the
            // moment causes a panic, as the Executor might be dropping this
            // Stream while it is already executing the Task, so that would be a
            // concurrent execution of the same Task, which the RefCell in our
            // Executor prevents by panicking.
            ManuallyDrop::drop(&mut self.receiver);
        }
        let interval_ref = self.interval_ref;
        js! {
            const intervalRef = @{interval_ref};
            const intervalInfo = Module.STDWEB.acquire_js_reference(intervalRef);
            clearInterval(intervalInfo.id);
            intervalInfo.resolve.drop();
            Module.STDWEB.decrement_refcount(intervalRef);
        }
    }
}

impl Stream for IntervalStream {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(self.receiver.poll().expect("Unexpected cancel"))
    }
}

pub fn interval(duration: Duration) -> IntervalStream {
    stdweb::initialize();
    let milli = duration.as_secs() as f64 * 1_000.0 + duration.subsec_nanos() as f64 / 1_000_000.0;
    let (tx, rx) = unbounded();

    let resolve = move || {
        tx.unbounded_send(()).ok();
    };

    let interval_ref = js! {
        const resolve = @{resolve};
        const milli = @{milli};

        const intervalInfo = {
            id: null,
            resolve: resolve,
        };

        intervalInfo.id = setInterval(
            function() {
                intervalInfo.resolve();
            },
            milli
        );

        return Module.STDWEB.acquire_rust_reference(intervalInfo);
    }.try_into()
        .expect("Expected Reference to Interval Stream");

    IntervalStream {
        receiver: ManuallyDrop::new(rx),
        interval_ref,
    }
}
