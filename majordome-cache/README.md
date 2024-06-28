# majordome-cache

## Overview
This crates lets you cache any date in memory with a simple API.
To use it, simply inject **MajordomeCache** to your majordome App.
Then you can simply get a **MajordomeCacheGetter** by calling `MajordomeCache::key` with the key you want to cache.
```rust
async fn do_expensive_operation(query: Q) -> Result<R, E> {}

let resp = app
        .get::<MajordomeCache>()?
        .key(&query)
        .ttl(60)
        .try_get_with(do_expensive_operation(query))
        .await?;
```
The past example will cache the result of `do_expensive_operation` for 60 seconds in memory, with a key based on the query.
resp will have Arc<R> type, so you can clone it and share it across threads for cheap.