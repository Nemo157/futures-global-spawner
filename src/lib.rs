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
    static GLOBAL_SPAWNER: RefCell<Option<GlobalSpawner>> = RefCell::new(None);
}

#[derive(Clone)]
struct GlobalSpawner {
    spawner: Arc<dyn SharedSpawn + Send + Sync + 'static>,
}

impl GlobalSpawner {
    fn new(spawner: impl SharedSpawn + Send + Sync + 'static) -> GlobalSpawner {
        GlobalSpawner {
            spawner: Arc::new(spawner)
        }
    }
}

impl Spawn for &GlobalSpawner {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        let spawner = self.clone();
        self.spawner.spawn(async move {
            set_global_spawner(spawner);
            await!(future);
        })
    }

    fn status(&self) -> Result<(), SpawnError> {
        self.spawner.status()
    }
}

fn set_global_spawner(spawner: GlobalSpawner) {
    GLOBAL_SPAWNER.with(|global_spawner| global_spawner.replace(Some(spawner)));
}

fn with_global_spawner<R>(f: impl FnOnce(&GlobalSpawner) -> R) -> R {
    GLOBAL_SPAWNER.with(|global_spawner| {
        f(global_spawner.borrow().as_ref().unwrap())
    })
}

pub fn set_spawner<Sp: Send + Sync + 'static>(spawner: Sp) where for<'a> &'a Sp: Spawn {
    set_global_spawner(GlobalSpawner::new(spawner));
}

pub fn with_spawner<R>(f: impl FnOnce(&mut Spawn) -> R) -> R {
    with_global_spawner(|mut global_spawner| {
        f(&mut global_spawner)
    })
}

pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    with_global_spawner(|spawner| {
        spawner.spawn(fut).unwrap()
    })
}

pub fn spawn_with_handle<Fut: Future + Send + 'static>(fut: Fut) -> RemoteHandle<Fut::Output> where Fut::Output: Send {
    with_global_spawner(|mut spawner| {
        spawner.spawn_with_handle(fut).unwrap()
    })
}
