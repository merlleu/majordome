use crate::module::{AppModBuilder, AppModInitOptions};

#[derive(Debug, Clone)]
pub struct EnvEntry {
    pub key: String,
    pub value: String,
}

pub struct AppModConfigGetter<'a> {
    pub ns: Option<String>,
    pub bld: &'a mut AppModBuilder,
    pub name: String,
}

impl<'a> AppModConfigGetter<'a> {
    pub fn new<T>(o: &AppModInitOptions<T>, bld: &'a mut AppModBuilder, name: &str) -> Self {
        AppModConfigGetter {
            ns: o.ns.clone(),
            bld,
            name: name.to_string(),
        }
    }

    pub fn get_or<T>(&mut self, key: &str, default: &T) -> T
    where
        T: Clone + std::str::FromStr + std::fmt::Display,
    {
        let key = self.create_key(key);
        self.bld.register_env_entry(key.clone(), default.to_string());

        let s = match self.bld.app.config.get(&key) {
            Some(s) => s,
            None => return default.clone(),
        };

        match s.parse::<T>() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Failed to parse config value for key '{}'.", key);
                default.clone()
            }
        }
    }

    pub fn get_or_panic<T>(&mut self, key: &str) -> T
    where
        T: Clone + std::str::FromStr,
    {
        let key = self.create_key(key);
        self.bld.register_env_entry(key.clone(), "<REQUIRED>".to_string());

        let s = match self.bld.app.config.get(&key) {
            Some(s) => s,
            None => panic!("Config value for key '{}' not found.", key),
        };

        match s.parse::<T>() {
            Ok(v) => v,
            Err(_) => {
                panic!("Failed to parse config value for key '{}'.", key);
            }
        }
    }

    pub fn get_optional<T>(&mut self, key: &str) -> Option<T>
    where
        T: Clone + std::str::FromStr,
    {
        let key = self.create_key(key);
        self.bld.register_env_entry(key.clone(), "".to_string());

        let s = match self.bld.app.config.get(&key) {
            Some(s) => s,
            None => return None,
        };

        match s.parse::<T>() {
            Ok(v) => Some(v),
            Err(_) => {
                eprintln!("Failed to parse config value for key '{}'.", key);
                None
            }
        }
    }

    fn create_key(&self, key: &str) -> String {
        match &self.ns {
            Some(ns) => format!("{}_{}_{}", ns, self.name, key),
            None => format!("{}_{}", self.name, key),
        }
        .to_uppercase()
    }
}
