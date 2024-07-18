use async_trait::async_trait;
use majordome::{
    appmod_decl_self_pointer, AppMod, AppModBuilder, AppModConfigGetter, AppModInitOptions,
    AppModRuntime, MajordomeError,
};
use scylla::{prepared_statement::PreparedStatement, serialize::row::SerializeRow};
use std::sync::Arc;
use tokio::sync::Mutex;

pub use majordome_derive::ScyllaRow;

mod err;
mod traits;
pub use err::*;
pub use traits::*;

pub mod __private;

#[derive(Clone)]
pub struct ScyllaDB {
    inner: Arc<CachedScylla>,
}

struct CachedScylla {
    pub(crate) db: scylla::Session,
    pub(crate) cache: dashmap::DashMap<String, Arc<PreparedStatement>>,
    pub(crate) cache_hashed: dashmap::DashMap<u64, Arc<PreparedStatement>>,
    pub(crate) prepare_lock: Mutex<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScyllaDBConfig {
    pub hosts: Vec<String>,
    pub keyspace: String,
    pub auth: Option<ScyllaAuth>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScyllaAuth {
    pub username: String,
    pub password: String,
}

impl AppModRuntime for ScyllaDB {}

#[async_trait]
impl AppMod for ScyllaDB {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    type InitOptions = ();
    type ModConfig = ScyllaDBConfig;

    async fn config(
        builder: &mut AppModBuilder,
        opts: AppModInitOptions<Self::InitOptions>,
    ) -> Result<Self::ModConfig, MajordomeError> {
        let c = AppModConfigGetter::new(&opts, builder, "db.scylla");

        let hosts = c
            .get_or_panic::<String>("hosts")
            .split(',')
            .map(|s| s.to_string())
            .collect();
        let keyspace = c.get_or_panic("keyspace");

        let auth = match (c.get_optional("username"), c.get_optional("password")) {
            (Some(username), Some(password)) => Some(ScyllaAuth { username, password }),
            _ => None,
        };

        Ok(ScyllaDBConfig {
            hosts,
            keyspace,
            auth,
        })
    }

    async fn init(
        _builder: &mut AppModBuilder,
        config: Self::ModConfig,
    ) -> Result<Self, MajordomeError> {
        let mut db = scylla::SessionBuilder::new()
            .known_nodes(config.hosts)
            .use_keyspace(&config.keyspace, false)
            .compression(Some(scylla::transport::Compression::Snappy))
            .tcp_nodelay(true);

        if let Some(auth) = config.auth {
            db = db.user(auth.username, auth.password);
        }

        Ok(ScyllaDB {
            inner: Arc::new(CachedScylla {
                db: db.build().await.expect("Failed to connect to ScyllaDB"),
                cache: Default::default(),
                cache_hashed: Default::default(),
                prepare_lock: Default::default(),
            }),
        })
    }
}

impl ScyllaDB {
    /// Attempts to retrieve a prepared statement from the cache, or prepares it if it's not found.
    pub async fn prepare(
        &self,
        query: &str,
    ) -> Result<Arc<PreparedStatement>, ::scylla::transport::errors::QueryError> {
        let prepared = match self.inner.cache.get(query) {
            Some(e) => {
                let e = e.clone();
                e
            }
            None => {
                // lock the cache
                let lock = self.inner.prepare_lock.lock().await;

                // check again
                let r = match self.inner.cache.get(query) {
                    Some(e) => {
                        let e = e.clone();
                        e
                    }
                    None => {
                        #[cfg(debug_assertions)]
                        println!("🔍 Preparing CQL: {:?}.", query);

                        let prep = self.inner.db.prepare(query.to_string()).await?;
                        let e = std::sync::Arc::new(prep);
                        self.inner.cache.insert(query.to_string(), e.clone());
                        e
                    }
                };

                drop(lock);

                r
            }
        };

        Ok(prepared)
    }

    /// Attempts to retrieve a prepared statement from the cache, or prepares it if it's not found.
    /// The query is generated by the closure if the statement is not found.
    /// This is useful when generating the query string is expensive.
    pub async fn prepare_by_hash_or(
        &self,
        hash: u64,
        query: impl FnOnce() -> String,
    ) -> Result<Arc<PreparedStatement>, ::scylla::transport::errors::QueryError> {
        let prepared = match self.inner.cache_hashed.get(&hash) {
            Some(e) => {
                let e = e.clone();
                e
            }
            None => {
                // lock the cache
                let lock = self.inner.prepare_lock.lock().await;

                // check again
                let r = match self.inner.cache_hashed.get(&hash) {
                    Some(e) => {
                        let e = e.clone();
                        e
                    }
                    None => {
                        let generated_query = query();

                        #[cfg(debug_assertions)]
                        println!("🔍 Preparing CQL by hash ({hash:?}): {generated_query:?}.");

                        let prep = self.inner.db.prepare(generated_query).await?;
                        let e = std::sync::Arc::new(prep);
                        self.inner.cache_hashed.insert(hash, e.clone());
                        e
                    }
                };

                drop(lock);

                r
            }
        };

        Ok(prepared)
    }

    /// Execute a prepared statement with values.
    pub async fn execute(
        &self,
        prepared: &PreparedStatement,
        values: impl SerializeRow,
    ) -> Result<scylla::QueryResult, ::scylla::transport::errors::QueryError> {
        let result = self.inner.db.execute(prepared, values).await?;
        Ok(result)
    }
    /// Execute a non-prepared query with values.
    /// Attempts to retrieve a prepared statement from the cache, or prepares it if it's not found.
    pub async fn query(
        &self,
        query: &str,
        values: impl SerializeRow,
    ) -> Result<scylla::QueryResult, ::scylla::transport::errors::QueryError> {
        let prepared = self.prepare(query).await?;
        let result = self.inner.db.execute(&prepared, values).await?;
        Ok(result)
    }
}

appmod_decl_self_pointer!(ScyllaDB);
