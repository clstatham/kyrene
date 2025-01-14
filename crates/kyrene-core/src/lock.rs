pub type Mutex<T> = tokio::sync::Mutex<T>;
pub type MutexGuard<'a, T> = tokio::sync::MutexGuard<'a, T>;
pub type MappedMutexGuard<'a, T> = tokio::sync::MappedMutexGuard<'a, T>;

pub type RwLock<T> = tokio::sync::RwLock<T>;
pub type RwLockReadGuard<'a, T> = tokio::sync::RwLockReadGuard<'a, T>;
pub type RwLockWriteGuard<'a, T> = tokio::sync::RwLockWriteGuard<'a, T>;
pub type RwLockMappedWriteGuard<'a, T> = tokio::sync::RwLockMappedWriteGuard<'a, T>;
