use std::{
    any::TypeId,
    fmt::Debug,
    hash::{BuildHasherDefault, Hash, Hasher},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy)]
pub struct TypeInfo {
    type_id: TypeId,
    type_name: &'static str,
}

impl TypeInfo {
    pub fn of<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

impl Debug for TypeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.type_name, f)
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for TypeInfo {}

#[derive(Default)]
pub struct TypeIdHasher {
    state: u64,
}

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write_u128(&mut self, i: u128) {
        self.state = i as u64;
    }

    fn write_u64(&mut self, i: u64) {
        self.state = i;
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("TypeIdHasher should not be used with anything other than TypeId")
    }
}

#[derive(Clone)]
pub struct TypeIdMap<T>(hashbrown::HashMap<TypeInfo, T, BuildHasherDefault<TypeIdHasher>>);

impl<T> TypeIdMap<T> {
    pub fn new() -> Self {
        Self(hashbrown::HashMap::<
            TypeInfo,
            T,
            BuildHasherDefault<TypeIdHasher>,
        >::default())
    }

    pub fn insert_for<K: 'static>(&mut self, value: T) -> Option<T> {
        self.0.insert(TypeInfo::of::<K>(), value)
    }

    pub fn get_for<K: 'static>(&self) -> Option<&T> {
        self.0.get(&TypeInfo::of::<K>())
    }

    pub fn get_mut_for<K: 'static>(&mut self) -> Option<&mut T> {
        self.0.get_mut(&TypeInfo::of::<K>())
    }

    pub fn contains_type<K: 'static>(&self) -> bool {
        self.0.contains_key(&TypeInfo::of::<K>())
    }
}

impl<T> Default for TypeIdMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for TypeIdMap<T> {
    type Target = hashbrown::HashMap<TypeInfo, T, BuildHasherDefault<TypeIdHasher>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TypeIdMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// pub type TypeIdMap<T> = hashbrown::HashMap<TypeId, T, BuildHasherDefault<TypeIdHasher>>;
pub type TypeIdSet = hashbrown::HashSet<TypeId, BuildHasherDefault<TypeIdHasher>>;

pub type FxHashMap<K, V> = hashbrown::HashMap<K, V, rustc_hash::FxBuildHasher>;
pub type FxHashSet<T> = hashbrown::HashSet<T, rustc_hash::FxBuildHasher>;
