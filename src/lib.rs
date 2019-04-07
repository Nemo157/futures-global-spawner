#![feature(futures_api, async_await, await_macro)]

use futures_core::{task::{Spawn, SpawnError}, future::{Future, FutureObj}};
use futures_util::{task::SpawnExt, future::RemoteHandle};

use std::{cell::RefCell, sync::Arc};

trait SharedSpawn {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError>;

    fn status(&self) -> Result<(), SpawnError>;
}

impl<Sp> SharedSpawn for Sp where for<'a> &'a Sp: Spawn {
    fn spawn_obj(mut self: &Self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        Spawn::spawn_obj(&mut self, future)
    }

    fn status(&self) -> Result<(), SpawnError> {
        Spawn::status(&self)
    }
}

trait SharedSpawnExt: SharedSpawn {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> Result<(), SpawnError> {
        self.spawn_obj(FutureObj::new(Box::new(future)))
    }
}

impl<Sp: SharedSpawn + ?Sized> SharedSpawnExt for Sp {}

thread_local! {
    static GLOBAL_SPAWNER: RefCell<Option<Arc<dyn SharedSpawn + Send + Sync + 'static>>> = RefCell::new(None);
}

#[derive(Clone, Copy, Debug)]
struct GlobalSpawner;

impl Spawn for GlobalSpawner {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        GLOBAL_SPAWNER.with(|spawner| {
            let spawner = spawner.borrow();
            let spawner = spawner.as_ref().expect("global spawner was set");
            spawner.spawn({
                let spawner = spawner.clone();
                async move {
                    GLOBAL_SPAWNER.with(|global_spawner| global_spawner.replace(Some(spawner)));
                    await!(future);
                }
            })
        })
    }

    fn status(&self) -> Result<(), SpawnError> {
        GLOBAL_SPAWNER.with(|spawner| {
            spawner.borrow().as_ref().unwrap().status()
        })
    }
}

pub fn set_global_spawner<Sp: Send + Sync + 'static>(spawner: Sp) where for<'a> &'a Sp: Spawn {
    GLOBAL_SPAWNER.with(|global_spawner| global_spawner.replace(Some(Arc::new(spawner))));
}

pub fn spawner() -> impl Spawn {
    GlobalSpawner
}

pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) -> Result<(), SpawnError> {
    spawner().spawn(fut)
}

pub fn spawn_with_handle<Fut: Future + Send + 'static>(fut: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError> where Fut::Output: Send {
    spawner().spawn_with_handle(fut)
}
