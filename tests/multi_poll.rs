#![feature(async_await, await_macro, futures_api)]

use std::{thread, sync::{Arc, Mutex}, pin::Pin};

use futures::{future::{Future, FutureExt, FutureObj}, task::{SpawnExt, ArcWake, SpawnError, Spawn, Poll}};
use futures_test::future::FutureTestExt;

use futures_global_spawner;

#[test]
fn smoke() {
    futures_global_spawner::set_global_spawner(ThreadPerPoll::new());

    let result = futures_global_spawner::run(async {
        let a = futures_global_spawner::spawn_with_handle(async { 5 });
        let b = futures_global_spawner::spawner().spawn_with_handle(async { 6 }).unwrap();
        let (a, b) = await!(a.join(b));
        a + b
    }.pending_once());

    assert_eq!(result, 11);
}

#[derive(Clone, Copy)]
struct ThreadPerPoll;

struct ThreadPerPollFuture {
    future: Mutex<Option<FutureObj<'static, ()>>>,
}

impl ThreadPerPoll {
    fn new() -> Self {
        Self
    }
}

impl Spawn for &ThreadPerPoll {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        Arc::new(ThreadPerPollFuture { future: Mutex::new(Some(future)) }).into_waker().wake();
        Ok(())
    }
}

impl ArcWake for ThreadPerPollFuture {
    fn wake(arc_self: &Arc<Self>) {
        let arc_self = arc_self.clone();
        thread::spawn(move || {
            let waker = arc_self.clone().into_waker();
            let mut lock = arc_self.future.lock().unwrap();
            if let Some(future) = &mut *lock {
                if let Poll::Ready(()) = Pin::new(future).poll(&waker) {
                    drop(lock.take());
                }
            }
        });
    }
}
