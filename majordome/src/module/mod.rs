use std::{any::Any, fmt::Debug, hash::Hash, sync::Arc};

use crate::{MajordomeApp, MajordomeError};
use async_trait::async_trait;

pub mod builder;
pub use builder::*;

mod config;
pub use config::*;

mod store;
use tokio::{sync::Mutex, task::JoinHandle};

use self::store::{AnyMap, AnyMapByKey};

#[async_trait]
pub trait AppMod
where
    Self: Sized,
{
    type InitOptions: Hash + Clone + Send + Sync + Eq;
    const VERSION: &'static str = "unknown";

    // ModConfig is used to differentiate between different instances of the same module.
    // you can return () if you want to have only one instance of the module.
    // [!] do not have different values of this without a good reason.

    type ModConfig: Hash + Clone + Send + Sync + PartialEq + Eq + Debug;
    async fn config(
        builder: &mut AppModBuilder,
        opt: AppModInitOptions<Self::InitOptions>,
    ) -> Result<Self::ModConfig, MajordomeError>;

    async fn init(
        builder: &mut AppModBuilder,
        config: Self::ModConfig,
    ) -> Result<Self, MajordomeError>;
}

pub trait AppModPointer: Send + Sync + Clone + 'static {
    type Target: Any + Send + Sync + Clone + AppModRuntime + AppMod;
    fn opt(
        _builder: &mut AppModBuilder,
    ) -> AppModInitOptions<<<Self as AppModPointer>::Target as AppMod>::InitOptions> {
        AppModInitOptions {
            config: None,
            ns: None,
        }
    }
}

#[async_trait]
impl<T> AppMod for Arc<T>
where
    T: AppMod + Send + Sync,
{
    type InitOptions = T::InitOptions;
    type ModConfig = T::ModConfig;

    async fn config(
        builder: &mut AppModBuilder,
        opt: AppModInitOptions<Self::InitOptions>,
    ) -> Result<Self::ModConfig, MajordomeError> {
        T::config(builder, opt).await
    }

    async fn init(
        builder: &mut AppModBuilder,
        config: Self::ModConfig,
    ) -> Result<Self, MajordomeError> {
        T::init(builder, config).await.map(Arc::new)
    }
}

#[async_trait]
impl<T> AppModRuntime for Arc<T>
where
    T: AppModRuntime,
{
    async fn run(&self, app: MajordomeApp) -> Vec<AppModTask> {
        self.as_ref().run(app).await
    }

    async fn stop(&self, app: MajordomeApp) {
        self.as_ref().stop(app).await
    }
}

#[derive(Default, Hash)]
#[non_exhaustive]
pub struct AppModInitOptions<T> {
    pub config: Option<T>,
    pub ns: Option<String>,
}

#[async_trait]
pub trait AppModRuntime
where
    Self: Send + Sync,
{
    async fn run(&self, _app: MajordomeApp) -> Vec<AppModTask> {
        Vec::new()
    }
    async fn stop(&self, _app: MajordomeApp) {}
}

impl<T> AppModInitOptions<T> {
    pub fn new() -> Self {
        AppModInitOptions {
            config: None,
            ns: None,
        }
    }

    pub fn config(mut self, config: T) -> Self {
        self.config = Some(config);
        self
    }

    pub fn ns(mut self, ns: &str) -> Self {
        self.ns = Some(ns.to_string());
        self
    }
}

pub struct AppModTask {
    pub name: String,
    pub handle: JoinHandle<()>,
    pub wait: bool, // wether or not the process must wait for this task to finish before exiting.
    pub(crate) module_name: String,
    pub(crate) start_time: std::time::Instant,
}

impl AppModTask {
    pub fn new(handle: JoinHandle<()>) -> Self {
        AppModTask {
            name: "unknown".to_string(),
            handle,
            wait: true,
            module_name: "unknown".to_string(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn wait(mut self, wait: bool) -> Self {
        self.wait = wait;
        self
    }

    pub(crate) fn module_name(mut self, name: &str) -> Self {
        self.module_name = name.to_string();
        self
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

impl MajordomeApp {
    #[inline]
    pub fn get<T: AppModPointer + 'static>(&self) -> Result<&T::Target, MajordomeError>
    where
        T::Target: AppMod,
    {
        match self.modules.modules.get::<T>() {
            Some(m) => Ok(m),
            None => Err(MajordomeError::new(
                "errors.majordome.module_not_found".to_string(),
                format!("Module {} not found", get_type_name::<T::Target>()),
                vec![get_type_name::<T::Target>().to_string()],
                500,
            )),
        }
    }
}

#[derive(Default)]
pub(crate) struct ModuleStore {
    pub(crate) modules: AnyMapByKey, // Map<Type<T>, T::Target>
    pub(crate) modules_refs: Mutex<Vec<(String, Box<dyn AppModRuntime + Send + Sync>)>>,
    pub(crate) modules_targets_cache: AnyMap, // Map<(Type<T::Target>, Hash<InitOptions>), T::Target>

    pub(crate) handles: Mutex<Vec<AppModTask>>,
}

#[macro_export]
macro_rules! appmod_decl_ns_pointer {
    ($name:tt($target:ty): $ns:expr) => {
        #[derive(Clone)]
        pub struct $name;

        impl ::majordome::AppModPointer for $name {
            type Target = $target;
            fn opt(
                builder: &mut ::majordome::AppModBuilder,
            ) -> ::majordome::AppModInitOptions<
                <<Self as ::majordome::AppModPointer>::Target as ::majordome::AppMod>::InitOptions,
            > {
                ::majordome::AppModInitOptions::new().ns($ns)
            }
        }

        impl Into<$target> for $name {
            fn into(self) -> $target {
                panic!("This should never be called");
            }
        }
    };
}

#[macro_export]
macro_rules! appmod_decl_self_pointer {
    ($name:ty) => {
        impl ::majordome::AppModPointer for $name {
            type Target = Self;
        }
    };
}

#[macro_export]
macro_rules! appmod_decl_self_pointer_arc {
    ($name:ty) => {
        impl ::majordome::AppModPointer for $name {
            type Target = std::sync::Arc<Self>;
        }
    };
}

// appmod_decl_ns_pointer!(DefaultModulePointer, DefaultModule, "default");
// appmod_decl_self_pointer!(DefaultModule);

// #[async_trait]
// impl AppModPointer for DefaultModule {
//     type Target = Self;

//     async fn opt(builder: &mut crate::module::AppModBuilder) -> AppModInitOptions<()> {
//         AppModInitOptions {
//             ns: Some("default".to_string()),
//             ..Default::default()
//         }
//     }
// }
