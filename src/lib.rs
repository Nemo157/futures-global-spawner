#![feature(futures_api, async_await, await_macro)]

use futures_core::{task::{Spawn, SpawnError}, future::{Future, FutureObj}};
use futures_util::{task::SpawnExt, future::RemoteHandle};

use std::sync::RwLock;
use lazy_static::lazy_static;

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

lazy_static! {
    static ref GLOBAL_SPAWNER: RwLock<Option<Box<dyn SharedSpawn + Send + Sync + 'static>>> = RwLock::new(None);
}

#[derive(Clone, Copy, Debug)]
struct GlobalSpawner;

impl Spawn for GlobalSpawner {
    fn spawn_obj(&mut self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        let spawner = GLOBAL_SPAWNER.read().unwrap();
        let spawner = spawner.as_ref().expect("global spawner was set");
        spawner.spawn(future)
    }

    fn status(&self) -> Result<(), SpawnError> {
        GLOBAL_SPAWNER.read().unwrap().as_ref().unwrap().status()
    }
}

pub fn set_global_spawner<Sp: Send + Sync + 'static>(spawner: Sp) where for<'a> &'a Sp: Spawn {
    GLOBAL_SPAWNER.write().unwrap().replace(Box::new(spawner));
}

pub fn spawner() -> impl Spawn {
    GlobalSpawner
}

pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    spawner().spawn(fut).unwrap()
}

pub fn spawn_with_handle<Fut: Future + Send + 'static>(fut: Fut) -> RemoteHandle<Fut::Output> where Fut::Output: Send {
    spawner().spawn_with_handle(fut).unwrap()
}

pub fn run<Fut: Future + Send + 'static>(fut: Fut) -> Fut::Output where Fut::Output: Send {
    let (tx, rx) = std::sync::mpsc::channel();
    spawn(async move {
        let value = await!(fut);
        tx.send(value).unwrap();
    });
    rx.recv().unwrap()
}
