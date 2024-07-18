use std::{
    any::{Any, TypeId},
    hash::Hash,
    sync::Arc,
};

use async_trait::async_trait;
use majordome::{
    appmod_decl_self_pointer, AppMod, AppModBuilder, AppModConfigGetter, AppModInitOptions,
    AppModRuntime, MajordomeError,
};
use moka::future::Cache;

use crate::{expiry::MajordomeExpiry, getter::MajordomeCacheGetter};

pub(crate) type CacheKey = (TypeId, u64);

#[derive(Clone)]
pub struct CacheValue {
    pub value: Arc<dyn Any + Send + Sync>,
    pub ttl: u64,
}

#[derive(Clone)]
pub struct MajordomeCache {
    pub response_cache: Cache<CacheKey, CacheValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheConfig {
    pub max_size: u64,
}

impl AppModRuntime for MajordomeCache {}

#[async_trait]
impl AppMod for MajordomeCache {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    type InitOptions = ();
    type ModConfig = CacheConfig;

    async fn config(
        builder: &mut AppModBuilder,
        opts: AppModInitOptions<Self::InitOptions>,
    ) -> Result<Self::ModConfig, MajordomeError> {
        let c = AppModConfigGetter::new(&opts, builder, "majordome_cache");

        Ok(CacheConfig {
            max_size: c.get_or("max_size", &1000),
        })
    }

    async fn init(
        _builder: &mut AppModBuilder,
        config: Self::ModConfig,
    ) -> Result<Self, MajordomeError> {
        let cache = Cache::builder()
            .max_capacity(config.max_size)
            .expire_after(MajordomeExpiry)
            .build();
        Ok(MajordomeCache {
            response_cache: cache,
        })
    }
}

impl MajordomeCache {
    /// Create a new cache getter for key.
    /// The cache takes into account the type of the return value and the key.
    /// By default, the cache will not expire. You can set the expiration time with the ttl method.
    /// ```rust
    ///
    /// async fn your_async_getter() -> Result<String, ()> {
    ///    tokio::time::sleep(Duration::from_secs(5)).await;
    ///    Ok("value".to_string())
    /// }
    ///
    /// let cache = app.get::<MajordomeCache>()?;
    /// cache.key(("key1", 1, 5)).ttl(60).try_get_with(your_async_getter).await?;
    ///
    /// ```
    pub fn key<T: Hash>(&self, key: T) -> MajordomeCacheGetter {
        MajordomeCacheGetter::new(self, key)
    }
}

appmod_decl_self_pointer!(MajordomeCache);
