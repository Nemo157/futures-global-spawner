#![feature(async_await, await_macro, futures_api)]

#[test]
fn panic_when_unset() {
    let result = std::panic::catch_unwind(|| {
        futures_global_spawner::run(async { 11 });
    });
    assert_eq!(
        result.unwrap_err().downcast_ref::<&'static str>().unwrap(),
        &"global spawner not configured");
}
