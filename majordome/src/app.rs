use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use crate::signal::MajordomeSignal;
use crate::{AppModBuilder, ModuleStore};

pub struct MajordomeAppInner {
    // Configuration values, gathered from the environment.
    pub config: HashMap<String, String>,

    // Modules store.
    pub(crate) modules: ModuleStore,

    pub(crate) signal: MajordomeSignal,
}

#[derive(Clone)]
pub struct MajordomeApp {
    pub(crate) inner: Arc<MajordomeAppInner>,
}

impl MajordomeApp {
    /// Get reference to inner app data.
    pub fn get_ref(&self) -> &MajordomeAppInner {
        self.inner.as_ref()
    }

    /// Convert to the internal Arc<T>
    pub fn into_inner(self) -> Arc<MajordomeAppInner> {
        self.inner
    }
}

impl Deref for MajordomeApp {
    type Target = Arc<MajordomeAppInner>;

    fn deref(&self) -> &Arc<MajordomeAppInner> {
        &self.inner
    }
}

fn get_config() -> HashMap<String, String> {
    let mut m = HashMap::new();
    for (k, v) in std::env::vars() {
        m.insert(k, v);
    }

    println!("✅ Loaded {} configuration entries from env.", m.len());
    m
}

impl MajordomeApp {
    pub async fn new() -> MajordomeApp {
        let a = MajordomeApp {
            inner: Arc::new(Self::init().await),
        };
        a._start_exiting_probe();

        a
    }

    pub(crate) async fn init() -> MajordomeAppInner {
        let config = get_config();
        let signal = MajordomeSignal::new();

        MajordomeAppInner {
            config,
            modules: ModuleStore::default(),
            signal,
        }
    }

    pub async fn builder() -> AppModBuilder {
        AppModBuilder {
            app: Self::init().await,
            loadchain: Vec::new(),
            loaded: HashMap::new(),
            loaded_targets_count: 0,
        }
    }
}
