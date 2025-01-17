use std::{
    any::TypeId,
    fmt::Debug,
    hash::{BuildHasherDefault, Hash, Hasher},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy)]
pub struct TypeInfo {
    pub type_id: TypeId,
    #[cfg(debug_assertions)]
    pub type_name: &'static str,
}

impl TypeInfo {
    pub fn of<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            #[cfg(debug_assertions)]
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
        #[cfg(debug_assertions)]
        {
            write!(f, "{}", self.type_name)
        }
        #[cfg(not(debug_assertions))]
        {
            write!(f, "{:?}", self.type_id)
        }
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
        Self(hashbrown::HashMap::default())
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

// pub type TypeIdSet = hashbrown::HashSet<TypeInfo, BuildHasherDefault<TypeIdHasher>>;

#[derive(Clone, Debug, Default)]
pub struct TypeIdSet(hashbrown::HashSet<TypeInfo, BuildHasherDefault<TypeIdHasher>>);

impl TypeIdSet {
    pub fn insert_for<T: 'static>(&mut self) -> bool {
        self.0.insert(TypeInfo::of::<T>())
    }

    pub fn remove_for<T: 'static>(&mut self) -> bool {
        self.0.remove(&TypeInfo::of::<T>())
    }

    pub fn contains_type<T: 'static>(&self) -> bool {
        self.0.contains(&TypeInfo::of::<T>())
    }
}

impl Deref for TypeIdSet {
    type Target = hashbrown::HashSet<TypeInfo, BuildHasherDefault<TypeIdHasher>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TypeIdSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for TypeIdSet {
    type Item = TypeInfo;
    type IntoIter =
        <hashbrown::HashSet<TypeInfo, BuildHasherDefault<TypeIdHasher>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub type FxHashMap<K, V> = hashbrown::HashMap<K, V, rustc_hash::FxBuildHasher>;
pub type FxHashSet<T> = hashbrown::HashSet<T, rustc_hash::FxBuildHasher>;

#[macro_export]
macro_rules! define_atomic_id {
    ($id:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $id(u64);

        impl $id {
            pub const INVALID: Self = Self(u64::MAX);

            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                Self(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
            }

            pub fn is_valid(&self) -> bool {
                *self != Self::INVALID
            }

            pub const fn from_u64(id: u64) -> Self {
                Self(id)
            }

            pub const fn as_u64(&self) -> u64 {
                self.0
            }

            pub const fn as_usize(&self) -> usize {
                self.0 as usize
            }
        }

        impl Into<u64> for $id {
            fn into(self) -> u64 {
                self.0
            }
        }

        impl Into<usize> for $id {
            fn into(self) -> usize {
                self.0 as usize
            }
        }
    };
}
