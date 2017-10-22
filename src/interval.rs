use std::time::Duration;
use stdweb::unstable::TryInto;
use futures::{task, Async, Poll};
use {stdweb, Stream};

pub struct IntervalStream(f64);

impl Drop for IntervalStream {
    fn drop(&mut self) {
        // println!("Drop stream");
        let wrapped_stream_id = self.0;
        js! {
            const wrappedStream = Module.STDWEB.acquire_js_reference(@{wrapped_stream_id});
            clearInterval(wrappedStream.intervalId);
            Module.STDWEB.decrement_refcount(@{wrapped_stream_id});
        }
    }
}

impl Stream for IntervalStream {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let wrapped_stream_id = self.0;

        let notify_count: f64 = js! {
            const wrappedStream = Module.STDWEB.acquire_js_reference(@{wrapped_stream_id});
            return wrappedStream.notifyCount;
        }.try_into()
            .expect("Expected Notify Count for Interval Stream");

        if notify_count > 0.0 {
            js! {
                const wrappedStream = Module.STDWEB.acquire_js_reference(@{wrapped_stream_id});
                wrappedStream.notifyCount -= 1;
            }
            Ok(Async::Ready(Some(())))
        } else {
            let task = task::current();
            let notify = move || {
                task.notify();
            };
            js! {
                const wrappedStream = Module.STDWEB.acquire_js_reference(@{wrapped_stream_id});
                const notify = @{notify};
                wrappedStream.notifier = notify;
            }
            Ok(Async::NotReady)
        }
    }
}

pub fn interval(duration: Duration) -> IntervalStream {
    stdweb::initialize();
    let milli = duration.as_secs() as f64 * 1_000.0 + duration.subsec_nanos() as f64 / 1_000_000.0;

    let wrapped_stream_id: f64 = js! {
        const wrappedStream = {
            notifyCount: 0,
            intervalId: null,
            notifier: null,
        };
        const milli = @{milli};

        wrappedStream.intervalId = setInterval(
            function() {
                wrappedStream.notifyCount += 1;
                const notifier = wrappedStream.notifier;
                if (notifier) {
                    wrappedStream.notifier = null;
                    notifier();
                    notifier.drop();
                }
            },
            milli
        );

        return Module.STDWEB.acquire_rust_reference(wrappedStream);
    }.try_into()
        .expect("Expected Reference to Interval Stream");

    IntervalStream(wrapped_stream_id)
}
