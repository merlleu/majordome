use super::AppMod;
use crate::{
    AppModInitOptions, AppModPointer, AppModRuntime, AppModTask, MajordomeApp, MajordomeAppInner,
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::Arc,
};

pub struct AppModBuilder {
    pub app: MajordomeAppInner,

    pub(crate) loadchain: Vec<String>,
    pub(crate) loaded: HashMap<String, HashMap<TypeId, HashSet<u64>>>, // name -> (typeid, ConfigHash)
    pub(crate) loaded_targets_count: usize,
}

impl AppModBuilder {
    pub async fn add<P: AppModPointer + 'static>(mut self) -> Self
    where
        P::Target: AppMod + Send + Sync,
    {
        self.load::<P>().await;
        self
    }

    pub async fn load<P: AppModPointer + 'static>(&mut self) -> P::Target {
        match self.app.modules.modules.get::<P>() {
            Some(module) => {
                println!(
                    "{} | Pointer module already loaded {}",
                    self.repr_loadchain(),
                    repr_pointer_type::<P>()
                );
                module.clone()
            }
            None => {
                let opts = P::opt(self);
                let module = self.load_target_module::<P::Target, P>(opts).await;
                self.app.modules.modules.insert::<P>(module.clone());
                module
            }
        }
    }

    async fn load_target_module<
        M: AppModRuntime + AppMod + Clone + 'static,
        P: AppModPointer + 'static,
    >(
        &mut self,
        opts: AppModInitOptions<M::InitOptions>,
    ) -> M {
        let config = M::config(self, opts).await.expect(
            format!(
                "{} | Failed to load config for module {}",
                self.repr_loadchain(),
                repr_pointer_type::<P>()
            )
            .as_str(),
        );

        #[cfg(debug_assertions)]
        println!(
            "{} | Loading module {}, config: {:?} (hash: {:0x})",
            self.repr_loadchain(),
            repr_pointer_type::<P>(),
            config,
            hash_config(&config)
        );

        match self.get_target_module_cache::<M>(&config) {
            Some(module) => {
                #[cfg(debug_assertions)]
                println!(
                    "{} | Target module already loaded {}",
                    self.repr_loadchain(),
                    repr_pointer_type::<P>()
                );
                module.clone()
            }
            None => {
                self.push_chain::<P, M::ModConfig>(&config);

                let module = M::init(self, config.clone()).await.expect(
                    format!(
                        "{} | Failed to initialize module {}",
                        self.repr_loadchain(),
                        repr_pointer_type::<P>()
                    )
                    .as_str(),
                );

                self.insert_target_module_cache(config, module.clone());
                self.loaded_targets_count += 1;
                self.app
                    .modules
                    .modules_refs
                    .lock()
                    .await
                    .push((repr_pointer_type::<P>(), Box::new(module.clone())));

                self.loadchain.pop();

                println!(
                    "{} | Loaded target module {}",
                    self.repr_loadchain(),
                    repr_pointer_type::<P>(),
                );

                module
            }
        }
    }

    fn get_target_module_cache<M: AppModRuntime + AppMod + Clone + 'static>(
        &self,
        cfg: &M::ModConfig,
    ) -> Option<M> {
        self.app
            .modules
            .modules_targets_cache
            .get::<HashMap<M::ModConfig, M>>()?
            .get(cfg)
            .cloned()
    }

    fn insert_target_module_cache<M: AppModRuntime + AppMod + Clone + 'static>(
        &mut self,
        cfg: M::ModConfig,
        module: M,
    ) {
        // due to lack of entry/get_mut, we just remove update then reinsert
        let mut old = *self
            .app
            .modules
            .modules_targets_cache
            .remove::<HashMap<M::ModConfig, M>>()
            .unwrap_or_default();

        old.insert(cfg, module);
        self.app.modules.modules_targets_cache.insert(old);
    }

    pub async fn build(self) -> MajordomeApp {
        println!(
            "🏁 Loaded {} modules ({} pointers).",
            self.loaded_targets_count,
            self.app.modules.modules.len()
        );
        let a = MajordomeApp {
            inner: Arc::new(self.app),
        };
        a._start_exiting_probe();

        load_modules(a.clone()).await;
        a
    }

    pub fn exists<T: AppModPointer + 'static>(&self) -> bool {
        self.app.modules.modules.contains::<T>()
    }

    fn push_chain<P: AppModPointer + 'static, C: Hash + 'static>(&mut self, config: &C) {
        let instances_by_name = self
            .loaded
            .entry(get_type_name::<P::Target>().to_string())
            .or_default();

        let instances_by_type = instances_by_name
            .entry(TypeId::of::<P::Target>())
            .or_default();

        let hash = hash_config(config);
        if instances_by_type.contains(&hash) {
            panic!(
                "{} | Module {} already loaded with config hash {}",
                self.repr_loadchain(),
                repr_pointer_type::<P>(),
                hash
            );
        }

        instances_by_type.insert(hash);

        if instances_by_name.len() > 1 {
            let len = instances_by_name.len();

            println!(
                "{} | Found {} instances of target-module {}",
                self.repr_loadchain(),
                len,
                repr_pointer_type::<P>()
            );
        }

        self.loadchain.push(repr_pointer_type::<P>());
    }

    fn repr_loadchain(&self) -> String {
        let mut repr = String::new();

        for (i, r) in self.loadchain.iter().enumerate() {
            if i != 0 {
                repr.push_str(" -> ");
            }

            repr.push_str(r);
        }

        repr
    }
}

async fn load_modules(app: MajordomeApp) {
    let mut tasks = Vec::new();

    for (name, module) in app.modules.modules_refs.lock().await.iter() {
        let task = module.run(app.clone()).await;
        for task in task {
            tasks.push(task.module_name(name));
        }
    }

    let mut handles = app.get_ref().modules.handles.lock().await;
    for task in tasks {
        handles.push(task);
    }

    #[cfg(debug_assertions)]
    println!("Loaded {} tasks.", handles.len());
}

pub(crate) async fn stop_modules(app: MajordomeApp) {
    let mut tasks = Vec::new();

    let mut refs = app.modules.modules_refs.lock().await;
    let mut refs_new = Vec::new();
    std::mem::swap(&mut *refs, &mut refs_new);

    for (name, module) in refs_new {
        let app = app.clone();

        let task = AppModTask::new(tokio::spawn(async move {
            module.stop(app.clone()).await;
        }))
        .module_name(&name)
        .name("@stop");

        tasks.push(task);
    }

    // we collect the handles as we no longer need them globally after this.
    let mut handles = app.get_ref().modules.handles.lock().await;
    let mut handles_new = Vec::new();
    std::mem::swap(&mut *handles, &mut handles_new);

    for handle in handles_new {
        tasks.push(handle);
    }

    for task in tasks {
        match task.handle.await {
            Ok(_) => println!(
                "Task {} ({}) stopped successfully after {:?}",
                task.name,
                task.module_name,
                task.start_time.elapsed()
            ),
            Err(e) => {
                println!(
                    "Task {} ({}) failed to stop after {:?}: {:?}",
                    task.name,
                    task.module_name,
                    task.start_time.elapsed(),
                    e
                )
            }
        }
    }

    println!("👋 All modules stopped. Bye bye.");
}

fn hash_config<C: Hash + 'static>(cfg: &C) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    cfg.hash(&mut hasher);
    hasher.finish()
}

fn repr_pointer(pname: &str, name: &str, version: &str) -> String {
    if pname == name {
        return format!("{}={}", pname, version);
    }
    format!("{}/{}={}", pname, name, version)
}

fn repr_pointer_type<P: AppModPointer + 'static>() -> String {
    repr_pointer(
        get_type_name::<P>(),
        get_type_name::<P::Target>(),
        P::Target::VERSION,
    )
}

pub(crate) fn get_type_name<T: 'static>() -> &'static str {
    std::any::type_name::<T>()
}
