use std::{fmt::Debug, future::Future, marker::PhantomData, path::PathBuf, sync::Arc};

use downcast_rs::{impl_downcast, DowncastSync};
use kyrene_core::{
    define_atomic_id,
    event::Event,
    handler::{Local, Res, ResMut},
    lock::{Read, RwLock, Write},
    plugin::Plugin,
    prelude::{error, tokio::task::JoinSet, World, WorldHandle},
    util::{FxHashMap, TypeInfo},
    world_handle::FromWorldHandle,
};

define_atomic_id!(AssetId);

pub trait Asset: DowncastSync {}
impl_downcast!(sync Asset);
impl<T: DowncastSync> Asset for T {}

pub struct DynAsset {
    pub(crate) type_id: TypeInfo,
    pub(crate) asset: Box<dyn Asset>,
}

impl DynAsset {
    pub fn new<T: Asset>(asset: T) -> Self {
        Self {
            type_id: TypeInfo::of::<T>(),
            asset: Box::new(asset),
        }
    }
}

impl Debug for DynAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.type_id.fmt(f)
    }
}

impl std::ops::Deref for DynAsset {
    type Target = dyn Asset;

    fn deref(&self) -> &Self::Target {
        &*self.asset
    }
}

impl std::ops::DerefMut for DynAsset {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.asset
    }
}

pub struct Handle<T: Asset> {
    id: AssetId,
    _marker: PhantomData<T>,
}

impl<T: Asset> Handle<T> {
    pub const INVALID: Self = Self::new(AssetId::INVALID);

    pub(crate) const fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn into_dyn(self) -> DynHandle {
        DynHandle::new::<T>(self.id)
    }

    pub fn try_from_dyn(handle: DynHandle) -> Option<Self> {
        if handle.type_id == TypeInfo::of::<T>() {
            Some(Self::new(handle.id))
        } else {
            None
        }
    }
}

impl<T: Asset> Clone for Handle<T> {
    #[allow(clippy::non_canonical_clone_impl)]
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: PhantomData,
        }
    }
}

impl<T: Asset> Copy for Handle<T> {}

impl<T: Asset> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Asset> Eq for Handle<T> {}

impl<T: Asset> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle<{:?}>({:?})", std::any::type_name::<T>(), self.id)
    }
}

pub struct DynHandle {
    id: AssetId,
    type_id: TypeInfo,
}

impl DynHandle {
    pub(crate) fn new<T: Asset>(id: AssetId) -> Self {
        Self {
            id,
            type_id: TypeInfo::of::<T>(),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }
}

impl Debug for DynHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynHandle<{:?}>({:?})", self.type_id, self.id)
    }
}

pub struct AssetRef<T: Asset> {
    pub(crate) inner: Read<Option<DynAsset>>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: Asset> std::ops::Deref for AssetRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap().asset.downcast_ref().unwrap()
    }
}

impl<T: Asset + Debug> Debug for AssetRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(self, f)
    }
}

pub struct AssetMut<T: Asset> {
    pub(crate) inner: Write<Option<DynAsset>>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T: Asset> std::ops::Deref for AssetMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap().asset.downcast_ref().unwrap()
    }
}

impl<T: Asset> std::ops::DerefMut for AssetMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap().asset.downcast_mut().unwrap()
    }
}

impl<T: Asset + Debug> Debug for AssetMut<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(self, f)
    }
}

#[derive(Default)]
pub struct Assets {
    assets: FxHashMap<AssetId, Arc<RwLock<Option<DynAsset>>>>,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_manual<T: Asset>(&mut self, asset: T, id: AssetId) -> Handle<T> {
        self.assets
            .insert(id, Arc::new(RwLock::new(Some(DynAsset::new(asset)))));
        Handle::new(id)
    }

    pub fn insert<T: Asset>(&mut self, asset: T) -> Handle<T> {
        let id = AssetId::new();
        self.assets
            .insert(id, Arc::new(RwLock::new(Some(DynAsset::new(asset)))));
        Handle::new(id)
    }

    pub async fn remove<T: Asset>(&mut self, handle: Handle<T>) -> Option<T> {
        let asset = self.assets.remove(&handle.id)?;
        let mut asset = asset.write().await;
        let asset = asset.take().unwrap();
        Some(*asset.asset.downcast().unwrap_or_else(|_| unreachable!()))
    }

    pub async fn get<T: Asset>(&self, handle: Handle<T>) -> Option<AssetRef<T>> {
        let asset = self.assets.get(&handle.id)?;
        let asset = asset.clone().read_owned().await;
        Some(AssetRef {
            inner: asset,
            _marker: PhantomData,
        })
    }

    pub async fn get_mut<T: Asset>(&mut self, handle: Handle<T>) -> Option<AssetMut<T>> {
        let asset = self.assets.get(&handle.id)?;
        let asset = asset.clone().write_owned().await;
        Some(AssetMut {
            inner: asset,
            _marker: PhantomData,
        })
    }
}

#[derive(Debug)]
pub enum LoadSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
    Existing(DynAsset),
}

impl From<PathBuf> for LoadSource {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

impl From<Vec<u8>> for LoadSource {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }
}

impl From<DynAsset> for LoadSource {
    fn from(asset: DynAsset) -> Self {
        Self::Existing(asset)
    }
}

impl From<&str> for LoadSource {
    fn from(path: &str) -> Self {
        Self::Path(PathBuf::from(path))
    }
}

pub trait Load: FromWorldHandle + Send + Sync + 'static {
    type Asset: Asset;
    type Error: std::error::Error + Send + Sync + 'static;

    fn load(
        &self,
        source: &LoadSource,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send;
}

pub struct LoadRequest<T: Asset> {
    handle: Handle<T>,
    source: LoadSource,
}

pub struct Loader<L: Load> {
    queue: Arc<RwLock<Vec<LoadRequest<L::Asset>>>>,
    _loader: PhantomData<L>,
}

impl<L: Load> Default for Loader<L> {
    fn default() -> Self {
        Self {
            queue: Arc::new(RwLock::new(Vec::new())),
            _loader: PhantomData,
        }
    }
}

impl<L: Load> Loader<L> {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn load(&self, source: impl Into<LoadSource>) -> Handle<L::Asset> {
        let handle = Handle::new(AssetId::new());
        self.queue.write().await.push(LoadRequest {
            handle,
            source: source.into(),
        });
        handle
    }
}

pub struct AssetLoaderPlugin<L: Load>(PhantomData<L>);

impl<L: Load> Default for AssetLoaderPlugin<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<L: Load> Plugin for AssetLoaderPlugin<L> {
    async fn build(self, world: &mut World) {
        if !world.has_resource::<Assets>() {
            world.insert_resource(Assets::new()).await;
        }

        if !world.has_resource::<Loader<L>>() {
            world.insert_resource(Loader::<L>::new()).await;
        }

        world.add_event_handler(load_assets::<L>);
    }
}

pub struct LoadAssets<T: Asset>(PhantomData<T>);

impl<T: Asset> Default for LoadAssets<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub struct AssetLoaded<T: Asset> {
    pub handle: Handle<T>,
}

pub trait WorldAssets {
    fn load_asset<L: Load>(
        &self,
        source: impl Into<LoadSource> + Send,
    ) -> impl Future<Output = Handle<L::Asset>> + Send;
}

impl WorldAssets for WorldHandle {
    async fn load_asset<L: Load>(&self, source: impl Into<LoadSource> + Send) -> Handle<L::Asset> {
        let source = source.into();
        let loader = self.get_resource::<Loader<L>>().await.unwrap();
        let handle = loader.load(source).await;
        self.fire_event(LoadAssets::<L::Asset>::default(), false)
            .await;
        handle
    }
}

async fn load_assets<L: Load>(
    _event: Event<LoadAssets<L::Asset>>,
    world: WorldHandle,
    loader: Res<Loader<L>>,
    l: Local<L>,
    mut assets: ResMut<Assets>,
) {
    if loader.queue.read().await.is_empty() {
        return;
    }

    let l = Arc::new(l);

    let mut join_set = JoinSet::new();

    let mut queue = loader.queue.write().await;

    for request in queue.drain(..) {
        let LoadRequest { handle, source } = request;

        if assets.get(handle).await.is_some() {
            continue;
        }

        let source = Arc::new(source);
        join_set.spawn({
            let l = l.clone();
            let world = world.clone();
            async move {
                let asset = l.get().await.load(&source).await;
                let asset = match asset {
                    Ok(asset) => asset,
                    Err(err) => {
                        error!("Failed to load asset for {:?}: {}", handle, err);
                        return None;
                    }
                };
                world.fire_event(AssetLoaded { handle }, false).await;
                Some((handle, asset))
            }
        });
    }

    drop(queue);

    let results = join_set.join_all().await;
    for (handle, asset) in results.into_iter().flatten() {
        assets.insert_manual(asset, handle.id());
    }
}
