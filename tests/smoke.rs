#![feature(async_await, await_macro, futures_api)]

use futures::{executor::ThreadPool, future::FutureExt, task::SpawnExt};

use futures_global_spawner::{set_global_spawner, spawner, spawn_with_handle, run};

#[test]
fn smoke() {
    set_global_spawner(ThreadPool::new().unwrap());

    let result = run(async {
        let a = spawn_with_handle(async { 5 });
        let b = spawner().spawn_with_handle(async { 6 }).unwrap();
        let (a, b) = await!(a.join(b));
        a + b
    });

    assert_eq!(result, 11);
}
