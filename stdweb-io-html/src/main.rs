#![feature(proc_macro, conservative_impl_trait, generators)]
#![no_main]

extern crate futures_await as futures;
extern crate stdweb_io;

use futures::prelude::*;

use stdweb_io::{fetch, interval, spawn, timeout};
use stdweb_io::http::Request;
use std::time::Duration;

#[no_mangle]
pub extern "C" fn do_stuff() {
    let f = async_block! {
        println!("Waiting 5 seconds");

        await!(timeout(Duration::from_secs(5)))?;

        println!("5 seconds done");

        let progress_report = async_block! {
            let mut counter = 0;

            #[async]
            for _ in interval(Duration::from_millis(100)) {
                counter += 100;
                println!("{}ms progressed", counter);
            }

            Ok(())
        };

        let request = async_block! {
            println!("Starting Request");

            await!(fetch(
                Request::get("https://splits.io/api/v3/games?search=sonic")
                    .body(())
                    .unwrap(),
            ).map_err(|_| ()))?;

            println!("Request finished");

            Ok::<(), ()>(())
        };

        await!(progress_report.select(request).map(|_| ()).map_err(|_| ()))?;

        Ok(())
    };

    spawn(f);
}
