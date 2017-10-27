use futures::unsync::oneshot::{channel, Receiver};
use futures::{Async, Future, Poll};
use {stdweb, Error, Request, Response};
use std::ops::{Deref, DerefMut};
use std::slice;

pub struct FetchFuture(Receiver<Result<Response<AsyncBody>, Error>>);

impl Future for FetchFuture {
    type Item = Response<AsyncBody>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let async = self.0.poll().expect("Unexpected cancel");
        match async {
            Async::Ready(Ok(v)) => Ok(Async::Ready(v)),
            Async::Ready(Err(e)) => Err(e),
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}

pub fn fetch<T>(request: Request<T>) -> FetchFuture {
    stdweb::initialize();

    let (tx, rx) = channel();
    let mut tx = Some(tx);

    let (parts, body) = request.into_parts();
    let uri = parts.uri.to_string();

    let resolve_ok =
        move |okay: bool, status_code: u16, headers: Vec<Vec<String>>, response_ref: f64| {
            let body = AsyncBody { response_ref };

            let mut builder = Response::builder();

            for header in &headers {
                builder.header(header[0].as_str(), header[1].as_str());
            }

            let response = builder.status(status_code).body(body);

            tx.take()
                .expect("Unexpected second resolve of fetch")
                .send(response)
                .ok();
        };

    let resolve_err = move || -> () {
        unimplemented!()
        // tx.take()
        //     .expect("Unexpected second resolve of fetch")
        //     .send(Ok(()))
        //     .ok();
    };

    js! {
        const uri = @{uri};
        const resolveOk = @{resolve_ok};
        const resolveErr = @{resolve_err};

        fetch(uri).then(
            function(res) {
                // console.log(res);
                const okay = res.ok;
                const status = res.status;
                const headers = Array.from(res.headers.entries());
                const responseRef = Module.STDWEB.acquire_rust_reference(res);
                resolveOk(okay, status, headers, responseRef);
                resolveOk.drop();
                resolveErr.drop();
            },
            function(err) {
                resolveErr();
                resolveOk.drop();
                resolveErr.drop();
            }
        );
    }

    FetchFuture(rx)
}

pub struct BodyData(*mut [u8]);

impl Drop for BodyData {
    fn drop(&mut self) {
        let ptr: f64 = self.as_ptr() as usize as f64;
        js! {
            _free(@{ptr});
        }
    }
}

impl Deref for BodyData {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { &*self.0 }
    }
}

impl DerefMut for BodyData {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { &mut *self.0 }
    }
}

#[derive(Debug)]
pub struct AsyncBody {
    response_ref: f64,
}

impl Drop for AsyncBody {
    fn drop(&mut self) {
        let response_ref = self.response_ref;
        js! {
            const responseRef = @{response_ref};
            Module.STDWEB.decrement_refcount(responseRef);
        }
    }
}

impl AsyncBody {
    pub fn get(self) -> BodyFuture {
        let (tx, rx) = channel();
        let mut tx = Some(tx);

        let resolve = move |ptr: f64, len: f64| {
            let buf = unsafe { slice::from_raw_parts_mut(ptr as usize as *mut u8, len as usize) };
            let buf = BodyData(buf);
            tx.take()
                .expect("Unexpected second resolve of body")
                .send(buf)
                .ok();
        };

        let response_ref = self.response_ref;

        js! {
            const resolve = @{resolve};
            const responseRef = @{response_ref};
            const response = Module.STDWEB.acquire_js_reference(responseRef);
            response.arrayBuffer().then(
                function(arrayBuffer) {
                    const len = arrayBuffer.byteLength;
                    const ptr = _malloc(len);
                    writeArrayToMemory(new Int8Array(arrayBuffer), ptr);
                    resolve(ptr, len);
                    resolve.drop();
                },
                function(error) {
                    resolve.drop();
                }
            );
        }

        BodyFuture(rx)
    }
}

pub struct BodyFuture(Receiver<BodyData>);

impl Future for BodyFuture {
    type Item = BodyData;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll().map_err(|_| ())
    }
}
