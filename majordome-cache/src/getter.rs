use std::{
    any::{Any, TypeId},
    future::Future,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
    time::SystemTime,
};

use crate::appmod::{CacheValue, MajordomeCache};

pub struct MajordomeCacheGetter<'a> {
    key: u64,
    ttl: u64,
    svc: &'a MajordomeCache,
}

#[derive(Clone)]
pub struct CacheItem<T> {
    value: Arc<T>,
    created_at: SystemTime,
    hit: bool,
}

impl<T> CacheItem<T> {
    pub fn hit(&self) -> bool {
        self.hit
    }

    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed().unwrap_or_default()
    }

    pub fn value(self) -> Arc<T> {
        self.value
    }
}

impl<'a> MajordomeCacheGetter<'a> {
    pub(crate) fn new<T: Hash>(svc: &'a MajordomeCache, key: T) -> Self {
        let key = hash_key(&key);
        MajordomeCacheGetter { key, svc, ttl: 0 }
    }

    pub fn ttl(mut self, ttl: u64) -> Self {
        self.ttl = ttl;
        self
    }

    pub async fn try_get_with_meta<T, E>(
        &self,
        future: impl Future<Output = Result<T, E>>,
    ) -> Result<CacheItem<T>, E>
    where
        T: 'static + Send + Sync,
        E: 'static + Clone + Send + Sync,
    {
        let nonce = rand::random::<u64>();
        let key = (TypeId::of::<T>(), self.key);
        let r = self
            .svc
            .response_cache
            .try_get_with(key, async {
                match future.await {
                    Ok(v) => Ok(CacheValue {
                        value: (Arc::new(v) as Arc<dyn Any + Send + Sync>),
                        ttl: self.ttl,
                        created_at: SystemTime::now(),
                        nonce,
                    }),
                    Err(e) => Err(e),
                }
            })
            .await;

        match r {
            Ok(v) => Ok(CacheItem {
                value: v.value.downcast::<T>().unwrap().clone(),
                created_at: v.created_at,
                hit: v.nonce != nonce,
            }),
            Err(e) => Err((*e).clone()),
        }
    }

    #[deprecated(note = "use try_get_with_meta instead")]
    pub async fn try_get_with<T, E>(
        &self,
        future: impl Future<Output = Result<T, E>>,
    ) -> Result<Arc<T>, E>
    where
        T: 'static + Send + Sync,
        E: 'static + Clone + Send + Sync,
    {
        let r = self.try_get_with_meta(future).await?.value();
        Ok(r)
    }
}

fn hash_key<T: Hash>(key: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
