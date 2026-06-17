use http_cache_reqwest::{CacheManager, HttpResponse};
use http_cache_semantics::CachePolicy;
use moka::future::Cache;
use moka::policy::EvictionPolicy;
use serde::{Deserialize, Serialize};
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, BoxError>;
use std::sync::Arc;

pub struct HttpCacheManager {
    pub cache: Arc<Cache<String, Store>>,
}

impl Default for HttpCacheManager {
    fn default() -> Self {
        Self::new(42)
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Store {
    response: HttpResponse,
    policy: CachePolicy,
}

impl HttpCacheManager {
    pub fn new(cache_size: u64) -> Self {
        let cache = Cache::builder()
            .eviction_policy(EvictionPolicy::lru())
            .max_capacity(cache_size)
            .build();
        Self {
            cache: Arc::new(cache),
        }
    }

    pub async fn clear(&self) -> Result<()> {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
        Ok(())
    }
}

#[async_trait::async_trait]
impl CacheManager for HttpCacheManager {
    async fn get(&self, cache_key: &str) -> Result<Option<(HttpResponse, CachePolicy)>> {
        let store: Store = match self.cache.get(cache_key).await {
            Some(d) => d,
            None => return Ok(None),
        };
        Ok(Some((store.response, store.policy)))
    }

    async fn put(
        &self,
        cache_key: String,
        response: HttpResponse,
        policy: CachePolicy,
    ) -> Result<HttpResponse> {
        let data = Store {
            response: response.clone(),
            policy,
        };
        self.cache.insert(cache_key, data).await;
        self.cache.run_pending_tasks().await;
        Ok(response)
    }

    async fn delete(&self, cache_key: &str) -> Result<()> {
        self.cache.invalidate(cache_key).await;
        self.cache.run_pending_tasks().await;
        Ok(())
    }
}

/*#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;
    use http::Response;
    use http_cache::HttpVersion;
    use reqwest::{Method, ResponseBuilderExt, StatusCode};
    use reqwest::header::HeaderMap;
    use url::Url;

    use super::*;

    impl http_cache_semantics::ResponseLike for http::Response<bytes::Bytes> {
        fn status(&self) -> StatusCode {
            StatusCode::from_str(self.status().as_str()).unwrap()
        }

        fn headers(&self) -> &HeaderMap {
            todo!()
        }
    }

    fn convert_response(response: HttpResponse) -> Result<Response<bytes::Bytes>> {
        let ret_res = http::Response::builder()
            .status(response.status)
            .version(
                match response.version {
                    HttpVersion::Http09 => http::Version::HTTP_09,
                    HttpVersion::Http10 => http::Version::HTTP_10,
                    HttpVersion::Http11 => http::Version::HTTP_11,
                    HttpVersion::H2 => http::Version::HTTP_2,
                    HttpVersion::H3 => http::Version::HTTP_3,
                    _ => unreachable!()
                }
            )
            .body(response.body)?;
        let (parts, body) = ret_res.into_parts();

        Ok(Response::from_parts(parts, bytes::Bytes::from(body)))
    }

    async fn insert_key_into_cache(manager: &HttpCacheManager, key: &str) {
        let request_url = "http://localhost:8080/test";
        let url = Url::parse(request_url).unwrap();

        let http_resp = HttpResponse {
            headers: HashMap::default(),
            body: vec![1, 2, 3],
            status: 200,
            url: url.clone(),
            version: HttpVersion::Http11,
        };
        let resp = convert_response(http_resp.clone()).unwrap();
        let request: reqwest::Request =
            reqwest::Request::new(Method::GET, request_url.parse().unwrap());

        let _ = manager
            .put(
                key.to_string(),
                http_resp,
                CachePolicy::new(&request, &resp),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_put() {
        let manager = HttpCacheManager::default();
        insert_key_into_cache(&manager, "test").await;
        assert!(manager.cache.contains_key("test"));
    }

    #[tokio::test]
    async fn test_get_when_key_present() {
        let manager = HttpCacheManager::default();
        insert_key_into_cache(&manager, "test").await;
        let value = manager.get("test").await.unwrap();
        assert!(value.is_some());
    }

    #[tokio::test]
    async fn test_get_when_key_not_present() {
        let manager = HttpCacheManager::default();
        let result = manager.get("test").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_when_key_present() {
        let manager = HttpCacheManager::default();
        insert_key_into_cache(&manager, "test").await;

        assert!(manager.cache.iter().count() as i32 == 1);
        let _ = manager.delete("test").await;
        assert!(manager.cache.iter().count() as i32 == 0);
    }

    #[tokio::test]
    async fn test_clear() {
        let manager = HttpCacheManager::default();
        insert_key_into_cache(&manager, "test").await;
        assert!(manager.cache.iter().count() as i32 == 1);
        let _ = manager.clear().await;
        assert!(manager.cache.iter().count() as i32 == 0);
    }

    #[tokio::test]
    async fn test_lru_eviction_policy() {
        let manager = HttpCacheManager::new(2);
        insert_key_into_cache(&manager, "test-1").await;
        insert_key_into_cache(&manager, "test-2").await;
        insert_key_into_cache(&manager, "test-10").await;

        let res = manager.get("test-1").await.unwrap();
        assert!(res.is_none());

        let res = manager.get("test-2").await.unwrap();
        assert!(res.is_some());

        let res = manager.get("test-10").await.unwrap();
        assert!(res.is_some());

        assert_eq!(manager.cache.entry_count(), 2);
    }
}*/
