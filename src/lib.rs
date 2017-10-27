#![recursion_limit = "128"]

pub extern crate futures;
pub extern crate http;
#[macro_use]
pub extern crate stdweb;

pub use http::{Error, Request, Response, Result};
pub use futures::{Future, Stream};

mod core;
mod fetch;
mod timeout;
mod interval;

pub use core::{spawn, spawn_deferred, spawn_deferred_fn, spawn_fn};
pub use fetch::{fetch, AsyncBody, BodyData, BodyFuture, FetchFuture};
pub use timeout::{defer, timeout, TimeoutFuture};
pub use interval::{interval, IntervalStream};
