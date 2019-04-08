#![feature(async_await, await_macro, futures_api)]

use futures::{executor::ThreadPool, future::FutureExt, task::SpawnExt};

#[test]
fn smoke() {
    futures_global_spawner::set_global_spawner(ThreadPool::new().unwrap());

    let result = futures_global_spawner::run(async {
        let a = futures_global_spawner::spawn_with_handle(async { 5 });
        let b = futures_global_spawner::spawner().spawn_with_handle(async { 6 }).unwrap();
        let (a, b) = await!(a.join(b));
        a + b
    });

    assert_eq!(result, 11);
}
