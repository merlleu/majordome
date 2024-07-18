use std::time::Duration;
use std::time::Instant;

use moka::Expiry;

use crate::appmod::{CacheKey, CacheValue};

pub struct MajordomeExpiry;

impl Expiry<CacheKey, CacheValue> for MajordomeExpiry {
    fn expire_after_create(
        &self,
        _key: &CacheKey,
        value: &CacheValue,
        _current_time: Instant,
    ) -> Option<Duration> {
        if value.ttl == 0 {
            None
        } else {
            Some(Duration::from_secs(value.ttl))
        }
    }
}
