use std::collections::HashMap;
use crate::app_ctx::AppCtx;
use crate::blueprint::{Server, Upstream};
use crate::ir::IoId;
use crate::target_runtime::cache::InMemoryCache;
use crate::target_runtime::TargetRuntime;
use crate::value::Value;
use anyhow::Error;
use derive_setters::Setters;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};
use dashmap::DashMap;

#[derive(Clone)]
pub struct CacheErr(String);

impl From<anyhow::Error> for CacheErr {
    fn from(value: Error) -> Self {
        CacheErr(value.to_string())
    }
}

impl From<CacheErr> for anyhow::Error {
    fn from(value: CacheErr) -> Self {
        anyhow::Error::msg(value.0)
    }
}

#[derive(Setters)]
pub struct RequestContext {
    pub server: Server,
    pub upstream: Upstream,
    pub min_max_age: Arc<Mutex<Option<i32>>>,
    pub cache_public: Arc<Mutex<Option<bool>>>,
    pub runtime: TargetRuntime,
    pub cache: DashMap<IoId, Value>,
    // pub cache: InMemoryCache<IoId, Value>,
    // pub cache: Dedupe<IoId, Result<Value, CacheErr>>,
}

impl RequestContext {
    pub fn new(target_runtime: TargetRuntime) -> RequestContext {
        RequestContext {
            server: Default::default(),
            upstream: Default::default(),
            min_max_age: Arc::new(Mutex::new(None)),
            cache_public: Arc::new(Mutex::new(None)),
            runtime: target_runtime,
            // cache: Dedupe::new(1, true),
            // cache: InMemoryCache::new(),
            cache: Default::default(),
        }
    }
    fn set_min_max_age_conc(&self, min_max_age: i32) {
        *self.min_max_age.lock().unwrap() = Some(min_max_age);
    }
    pub fn get_min_max_age(&self) -> Option<i32> {
        *self.min_max_age.lock().unwrap()
    }

    pub fn set_cache_public_false(&self) {
        *self.cache_public.lock().unwrap() = Some(false);
    }

    pub fn is_cache_public(&self) -> Option<bool> {
        *self.cache_public.lock().unwrap()
    }

    pub fn set_min_max_age(&self, max_age: i32) {
        let min_max_age_lock = self.get_min_max_age();
        match min_max_age_lock {
            Some(min_max_age) if max_age < min_max_age => {
                self.set_min_max_age_conc(max_age);
            }
            None => {
                self.set_min_max_age_conc(max_age);
            }
            _ => {}
        }
    }

    pub async fn cache_get(&self, key: &IoId) -> Result<Option<Value>, anyhow::Error> {
        self.runtime.cache.get(key).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn cache_insert(
        &self,
        key: IoId,
        value: Value,
        ttl: NonZeroU64,
    ) -> Result<(), anyhow::Error> {
        self.runtime.cache.set(key, value, ttl).await
    }
}

impl From<&AppCtx> for RequestContext {
    fn from(app_ctx: &AppCtx) -> Self {
        Self {
            server: app_ctx.blueprint.server.clone(),
            upstream: app_ctx.blueprint.upstream.clone(),
            min_max_age: Arc::new(Mutex::new(None)),
            cache_public: Arc::new(Mutex::new(None)),
            runtime: app_ctx.runtime.clone(),
            // cache: Dedupe::new(1, true),
            // cache: InMemoryCache::new(),
            cache: Default::default(),
        }
    }
}
