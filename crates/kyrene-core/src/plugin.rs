use std::future::Future;

use crate::world::World;

#[allow(unused)]
pub trait Plugin: 'static + Send + Sync {
    fn build(self, world: &mut World) -> impl Future<Output = ()>;
}
