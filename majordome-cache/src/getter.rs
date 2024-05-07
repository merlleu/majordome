use std::{any::{Any, TypeId}, future::Future, hash::{DefaultHasher, Hash, Hasher}, sync::Arc};

use crate::appmod::{CacheValue, MajordomeCache};

pub struct MajordomeCacheGetter<'a> {
    key: u64,
    ttl: u64,
    svc: &'a MajordomeCache
}

impl<'a> MajordomeCacheGetter<'a> {
    pub(crate) fn new<T: Hash>(svc: &'a MajordomeCache, key: T) -> Self {
        let key = hash_key(&key);
        MajordomeCacheGetter {
            key,
            svc,
            ttl: 0,
        }
    }

    pub fn ttl(mut self, ttl: u64) -> Self {
        self.ttl = ttl;
        self
    }

    pub async fn try_get_with<T, E>(&self, future: impl Future<Output = Result<T, E>>) -> Result<Arc<T>, E> 
    where
        T: 'static + Send + Sync,
        E: 'static + Clone + Send + Sync
    {
        let key = (TypeId::of::<T>(), self.key);
        let r = self.svc.response_cache.try_get_with(key, async {
            match future.await {
                Ok(v) => Ok(CacheValue {
                    value: (Arc::new(v) as Arc<dyn Any + Send + Sync>),
                    ttl: self.ttl
                }),
                Err(e) => Err(e)
            }
        }).await;

        match r {
            Ok(v) => Ok(v.value.downcast::<T>().unwrap().clone()),
            Err(e) => Err((*e).clone())
        }
    }
}


fn hash_key<T: Hash>(key: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}