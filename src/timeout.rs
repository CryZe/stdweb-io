use std::time::Duration;
use stdweb::unstable::TryInto;
use futures::{task, Async, Poll};
use {stdweb, Future};

pub struct TimeoutFuture(f64);

impl Drop for TimeoutFuture {
    fn drop(&mut self) {
        // println!("Drop future");
        let wrapped_promise_id = self.0;
        js! {
            Module.STDWEB.decrement_refcount(@{wrapped_promise_id});
        }
    }
}

impl Future for TimeoutFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let wrapped_promise_id = self.0;
        let state = js! {
            const wrappedPromise = Module.STDWEB.acquire_js_reference(@{wrapped_promise_id});
            return wrappedPromise.state;
        }.into_string()
            .expect("Promise state should be a string");

        match state.as_str() {
            "pending" => {
                let task = task::current();
                let notify = move || {
                    task.notify();
                };

                js! {
                    const wrappedPromise = Module.STDWEB.acquire_js_reference(@{wrapped_promise_id});
                    const notify = @{notify};
                    wrappedPromise.promise.then(
                        function() {
                            notify();
                            notify.drop();
                        },
                        function() {
                            notify();
                            notify.drop();
                        }
                    );
                }

                return Ok(Async::NotReady);
            }
            "fulfilled" => Ok(Async::Ready(())),
            "rejected" => unimplemented!(),
            s => unreachable!("Unexpected Fetch Promise state '{}'", s),
        }
    }
}

pub fn timeout(duration: Duration) -> TimeoutFuture {
    stdweb::initialize();
    let milli = duration.as_secs() as f64 * 1_000.0 + duration.subsec_nanos() as f64 / 1_000_000.0;
    let wrapped_promise_id: f64 = js! {
        const wrappedPromise = {
            promise: null,
            state: "pending",
            value: null,
        };
        const milli = @{milli};

        wrappedPromise.promise = new Promise(function(resolve) {
            setTimeout(resolve, milli);
        }).then(
            function(res) {
                wrappedPromise.value = res;
                wrappedPromise.state = "fulfilled";
            },
            function(err) {
                wrappedPromise.value = err;
                wrappedPromise.state = "rejected";
            }
        );

        return Module.STDWEB.acquire_rust_reference(wrappedPromise);
    }.try_into()
        .expect("Expected Reference to Timeout Promise");

    TimeoutFuture(wrapped_promise_id)
}

pub fn defer() -> TimeoutFuture {
    timeout(Duration::from_secs(0))
}
