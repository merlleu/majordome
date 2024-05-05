use crate::module::{AppModBuilder, AppModInitOptions};

pub struct AppModConfigGetter<'a> {
    pub ns: &'a Option<String>,
    pub bld: &'a AppModBuilder,
    pub name: String,
}

impl<'a> AppModConfigGetter<'a> {
    pub fn new<T>(o: &'a AppModInitOptions<T>, bld: &'a AppModBuilder, name: &str) -> Self {
        AppModConfigGetter {
            ns: &o.ns,
            bld,
            name: name.to_string(),
        }
    }

    pub fn get_or<T>(&self, key: &str, default: &T) -> T
    where
        T: Clone + std::str::FromStr,
    {
        let key = self.create_key(key);
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

    pub fn get_or_panic<T>(&self, key: &str) -> T
    where
        T: Clone + std::str::FromStr,
    {
        let key = self.create_key(key);
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

    fn create_key(&self, key: &str) -> String {
        match self.ns {
            Some(ns) => format!("{}_{}_{}", ns, self.name, key),
            None => format!("{}_{}", self.name, key),
        }
        .to_uppercase()
    }
}
