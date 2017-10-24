use futures::unsync::oneshot::{channel, Receiver};
use futures::{Async, Future, Poll};
use {stdweb, Error, Request, Response};

pub struct FetchFuture(Receiver<Result<Response<()>, Error>>);

impl Future for FetchFuture {
    type Item = Response<()>;
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

    let resolve_ok = move |status: String| {
        let response = Response::new(());

        tx.take()
            .expect("Unexpected second resolve of setTimeout")
            .send(Ok(response))
            .ok();
    };

    let resolve_err = move || -> () {
        unimplemented!()
        // tx.take()
        //     .expect("Unexpected second resolve of setTimeout")
        //     .send(Ok(()))
        //     .ok();
    };

    js! {
        const uri = @{uri};
        const resolveOk = @{resolve_ok};
        const resolveErr = @{resolve_err};

        fetch(uri).then(
            function(res) {
                resolveOk("Hello");
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
