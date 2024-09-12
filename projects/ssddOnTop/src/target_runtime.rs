use crate::target_runtime::http::NativeHttp;
use std::sync::Arc;
use crate::blueprint::Upstream;
use crate::ir::IoId;
use crate::value::Value;

#[derive(Clone)]
pub struct TargetRuntime {
    /// HTTP client for making standard HTTP requests.
    pub http: Arc<NativeHttp>,
    pub cache: Arc<cache::InMemoryCache<IoId, Value>>,
}

impl TargetRuntime {
    pub fn new(upstream: &Upstream) -> Self {
        let http = Arc::new(NativeHttp::init(upstream));
        let cache = Arc::new(cache::InMemoryCache::new());
        Self { http, cache }
    }
}

mod cache {
    use std::hash::Hash;
    use std::num::NonZeroU64;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    use ttl_cache::TtlCache;
    use crate::value::ToInner;

    pub struct InMemoryCache<K: Hash + Eq, V> {
        data: Arc<RwLock<TtlCache<K, V>>>,
    }

    const CACHE_CAPACITY: usize = 100000;

    impl<K: Hash + Eq, V: Clone> Default for InMemoryCache<K, V> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<K: Hash + Eq, V: Clone> InMemoryCache<K, V> {
        pub fn new() -> Self {
            InMemoryCache {
                data: Arc::new(RwLock::new(TtlCache::new(CACHE_CAPACITY))),
            }
        }
    }

    impl<K: Hash + Eq + Send + Sync, V: Send + Sync + ToInner<Inner=Inner>, Inner: Clone> InMemoryCache<K, V> {
        pub async fn set<'a>(&'a self, key: K, value: V, ttl: NonZeroU64) -> anyhow::Result<()> {
            let ttl = Duration::from_millis(ttl.get());
            self.data.write().unwrap().insert(key, value, ttl);
            Ok(())
        }

        pub async fn get<'a>(&'a self, key: &'a K) -> anyhow::Result<Option<Inner>> {
            let val = self.data.read().unwrap().get(key).map(|v| v.to_inner());
            Ok(val)
        }
    }
}

mod http {
    use bytes::Bytes;
    use crate::blueprint::Upstream;
    use crate::cache::HttpCacheManager;
    use http_cache_reqwest::{Cache, CacheMode, HttpCache, HttpCacheOptions};
    use reqwest::Client;
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use crate::http::response::Response;
    use anyhow::Result;

    pub struct NativeHttp {
        client: ClientWithMiddleware,
    }

    impl NativeHttp {
        pub fn init(upstream: &Upstream) -> Self {
            let mut client = ClientBuilder::new(Client::new());

            client = client.with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: HttpCacheManager::new(upstream.http_cache),
                options: HttpCacheOptions::default(),
            }));

            Self {
                client: client.build(),
            }
        }
        pub async fn execute(&self, mut request: reqwest::Request) -> Result<Response<Bytes>> {
            tracing::info!(
            "{} {} {:?}",
            request.method(),
            request.url(),
            request.version()
        );
            tracing::debug!("request: {:?}", request);
            let response = self.client.execute(request).await;
            tracing::debug!("response: {:?}", response);

            Ok(Response::from_reqwest(
                response?
                    .error_for_status()
                    .map_err(|err| err.without_url())?,
            )
                .await?)
        }
    }
}