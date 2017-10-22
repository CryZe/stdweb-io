use futures::{task, Async, Poll};
use stdweb::unstable::TryInto;
use {stdweb, Error, Future, Request};

pub struct FetchFuture(f64);

impl Drop for FetchFuture {
    fn drop(&mut self) {
        // println!("Drop future");
        let wrapped_promise_id = self.0;
        js! {
            Module.STDWEB.decrement_refcount(@{wrapped_promise_id});
        }
    }
}

impl Future for FetchFuture {
    type Item = String;
    type Error = Error;

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
            "fulfilled" => {
                let status_text = js! {
                    const wrappedPromise = Module.STDWEB.acquire_js_reference(@{wrapped_promise_id});
                    const response = wrappedPromise.value;
                    return response.statusText;
                }.into_string()
                    .expect("Status Text is not a String");
                Ok(Async::Ready(status_text))
            }
            "rejected" => unimplemented!(),
            s => unreachable!("Unexpected Fetch Promise state '{}'", s),
        }
    }
}

pub fn fetch<T>(request: Request<T>) -> FetchFuture {
    stdweb::initialize();

    let (parts, body) = request.into_parts();
    let uri = parts.uri.to_string();

    let wrapped_promise_id: f64 = js! {
        const wrappedPromise = {
            promise: null,
            state: "pending",
            value: null,
        };

        wrappedPromise.promise = fetch(@{uri})
            .then(
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
        .expect("Expected Reference to Fetch Promise");

    FetchFuture(wrapped_promise_id)
}
