use futures::unsync::oneshot::{channel, Receiver};
use futures::{Future, Poll};
use stdweb;
use std::time::Duration;

pub struct TimeoutFuture(Receiver<()>);

impl Future for TimeoutFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(self.0.poll().expect("Unexpected cancel"))
    }
}

pub fn timeout(duration: Duration) -> TimeoutFuture {
    stdweb::initialize();

    let milli = duration.as_secs() as f64 * 1_000.0 + duration.subsec_nanos() as f64 / 1_000_000.0;
    let (tx, rx) = channel();

    let mut tx = Some(tx);
    let resolve = move || {
        tx.take()
            .expect("Unexpected second resolve of setTimeout")
            .send(())
            .ok();
    };

    js! {
        const milli = @{milli};
        const resolve = @{resolve};

        setTimeout(
            function() {
                resolve();
                resolve.drop();
            },
            milli
        );
    }

    TimeoutFuture(rx)
}

pub fn defer() -> TimeoutFuture {
    timeout(Duration::from_secs(0))
}
