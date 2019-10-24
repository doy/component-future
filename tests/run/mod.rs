#![allow(dead_code)]

use futures::future::Future as _;

pub fn future<T: Send + 'static, E: Send + 'static>(
    fut: impl futures::future::Future<Item = T, Error = E> + Send + 'static,
) -> Result<T, E> {
    let (wchan, rchan) = std::sync::mpsc::channel();
    run(fut.then(move |res| {
        wchan.send(res).unwrap();
        futures::future::ok(())
    }));
    rchan.iter().next().unwrap()
}

pub fn stream<T: Send + 'static, E: Send + 'static>(
    stream: impl futures::stream::Stream<Item = T, Error = E> + Send + 'static,
) -> Result<Vec<T>, E> {
    let (wchan, rchan) = std::sync::mpsc::channel();
    let wchan_ok = wchan.clone();
    let wchan_err = wchan.clone();
    drop(wchan);
    run(stream
        .for_each(move |i| {
            wchan_ok.send(Ok(i)).unwrap();
            futures::future::ok(())
        })
        .map_err(move |e| {
            wchan_err.send(Err(e)).unwrap();
        }));
    rchan.iter().collect()
}

// replacement for tokio::run which keeps panics on the main thread (so that
// they don't get swallowed up and ignored if they happen on other threads)
fn run(
    fut: impl futures::future::Future<Item = (), Error = ()> + Send + 'static,
) {
    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
    runtime.block_on(fut).unwrap()
}
