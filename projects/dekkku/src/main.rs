use async_graphql::dataloader::*;
use async_graphql::http::GraphiQLSource;
use async_graphql::*;
use async_graphql_poem::*;
use bytes::Bytes;
use dashmap::DashMap;
use futures::future;
use poem::{listener::TcpListener, web::Html, *};
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::oneshot;
use tokio::task;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;

const ALL_USERS: &str = "http://localhost:3000/users";
const ALL_POSTS: &str = "http://localhost:3000/posts";

struct Store {
    posts: RwLock<HashMap<i32, Post>>,
    users: RwLock<HashMap<i32, User>>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            posts: RwLock::new(HashMap::new()),
            users: RwLock::new(HashMap::new()),
        }
    }
}

struct Query;

#[Object]
impl Query {
    async fn posts(
        &self,
        ctx: &Context<'_>,
    ) -> std::result::Result<Vec<Post>, async_graphql::Error> {
        let http_client = ctx.data_unchecked::<Arc<RequestBatcher>>();
        let mut posts: Vec<Post> = http_client.request(ALL_POSTS).await?;
        let store = ctx.data_unchecked::<Arc<Store>>();

        let should_query_user = ctx
            .field()
            .selection_set()
            .any(|field| field.name() == "user");

        if should_query_user {
            let are_posts_same = {
                let is_cache_empty = store.posts.read().unwrap().is_empty();
                if is_cache_empty {
                    let mut rng = rand::thread_rng();
                    let selected_posts: Vec<&Post> = posts.choose_multiple(&mut rng, 2).collect();
                    if selected_posts.len() == 2 {
                        let mut posts_writer = store.posts.write().unwrap();
                        for post in selected_posts {
                            posts_writer.insert(post.id, post.clone());
                        }
                    }
                    false
                } else {
                    let cached_posts: Vec<Post> =
                        store.posts.read().unwrap().values().cloned().collect();
                    let mut posts_writer = store.posts.write().unwrap();
                    cached_posts.iter().all(|post| {
                        if let Some(new_post) = posts.iter().find(|p| p.id == post.id) {
                            if post != new_post {
                                posts_writer.insert(new_post.id, new_post.clone());
                                false
                            } else {
                                true
                            }
                        } else {
                            false
                        }
                    })
                }
            };

            if are_posts_same {
                // Posts are the same, load the data from store and update the posts' user property.
                let store_users = store.users.read().unwrap();
                for post in posts.iter_mut() {
                    if let Some(cached_user) = store_users.get(&post.user_id) {
                        post.user = Some(cached_user.clone());
                    }
                }
            } else {
                let loader = ctx.data_unchecked::<DataLoader<UserLoader>>();
                let user_ids = posts.iter().map(|p| p.user_id).collect::<Vec<_>>();
                let users = loader.load_many(user_ids).await?;

                // let qp = posts
                //     .iter()
                //     .map(|p| p.user_id)
                //     .collect::<HashSet<_>>()
                //     .into_iter()
                //     .map(|user_id| format!("id={}", user_id))
                //     .collect::<Vec<_>>()
                //     .join("&");
                // let url = format!("{ALL_USERS}?{qp}");
                // let users: Vec<User> = http_client.request(&url).await?;
                // let users = users
                //     .into_iter()
                //     .map(|user| (user.id, user))
                //     .collect::<HashMap<_, _>>();
                let mut store_users = store.users.write().unwrap();
                for post in posts.iter_mut() {
                    if let Some(user) = users.get(&post.user_id) {
                        post.user = Some(user.clone());
                        store_users.insert(user.id, user.clone());
                    }
                }
            }
        }

        Ok(posts)
    }

    async fn post(
        &self,
        ctx: &Context<'_>,
        id: i32,
    ) -> std::result::Result<Option<Post>, async_graphql::Error> {
        let loader = ctx.data_unchecked::<DataLoader<PostLoader>>();
        let mut post = loader.load_one(id).await?;

        let should_query_user = ctx
            .field()
            .selection_set()
            .any(|field| field.name() == "user");

        if should_query_user {
            if let Some(post_ref) = post.as_mut() {
                let loader = ctx.data_unchecked::<DataLoader<UserLoader>>();
                let user = loader.load_one(post_ref.user_id).await?;
                post_ref.user = user;
            }
        }
        Ok(post)
    }

    async fn users(
        &self,
        ctx: &Context<'_>,
    ) -> std::result::Result<Vec<User>, async_graphql::Error> {
        let http_client = ctx.data_unchecked::<Arc<RequestBatcher>>();
        let users: Vec<User> = http_client.request(ALL_USERS).await?;
        Ok(users)
    }

    async fn user(
        &self,
        ctx: &Context<'_>,
        id: i32,
    ) -> std::result::Result<Option<User>, async_graphql::Error> {
        let loader = ctx.data_unchecked::<DataLoader<UserLoader>>();
        let user = loader.load_one(id).await?;
        Ok(user)
    }
}

#[derive(SimpleObject, Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
struct Post {
    id: i32,
    #[graphql(name = "userId")]
    user_id: i32,
    title: Option<String>,
    body: Option<String>,
    user: Option<User>,
}

#[derive(SimpleObject, Serialize, Deserialize, Debug, Clone, PartialEq)]
struct User {
    id: i32,
    name: Option<String>,
    username: Option<String>,
    email: Option<String>,
    address: Option<Address>,
    phone: Option<String>,
    website: Option<String>,
}

#[derive(SimpleObject, Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Address {
    zipcode: Option<String>,
    geo: Option<Geo>,
}

#[derive(SimpleObject, Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Geo {
    lat: Option<f64>,
    lng: Option<f64>,
}

struct PostLoader(Arc<RequestBatcher>);
struct UserLoader(Arc<RequestBatcher>);

impl Loader<i32> for PostLoader {
    type Value = Post;
    type Error = async_graphql::Error;

    async fn load(
        &self,
        keys: &[i32],
    ) -> std::result::Result<std::collections::HashMap<i32, Self::Value>, Self::Error> {
        let client = &self.0;

        // Create a vector to hold all the futures
        let futures: Vec<_> = keys
            .iter()
            .map(|&id| {
                let url = format!("{}/{}", ALL_POSTS, id);
                let client = client.clone();
                task::spawn(async move {
                    let post: Post = client.request(&url).await?;
                    Ok::<_, async_graphql::Error>((id, post))
                })
            })
            .collect();

        // Wait for all futures to complete
        let results = future::join_all(futures).await;

        // Collect results into a HashMap
        let mut result = std::collections::HashMap::new();
        for task_result in results {
            match task_result {
                Ok(Ok((id, post))) => {
                    result.insert(id, post);
                }
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(async_graphql::Error::new(format!("Task join error: {}", e))),
            }
        }

        Ok(result)
    }
}

impl Loader<i32> for UserLoader {
    type Value = User;
    type Error = async_graphql::Error;

    async fn load(
        &self,
        keys: &[i32],
    ) -> std::result::Result<std::collections::HashMap<i32, Self::Value>, Self::Error> {
        let mut result = std::collections::HashMap::new();
        let qp = keys
            .iter()
            .map(|id| format!("id={}", id))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!("{}?{}", ALL_USERS, qp);
        let users: Vec<User> = self.0.request(&url).await?;
        for user in users {
            result.insert(user.id, user);
        }
        Ok(result)
    }
}

async fn fetch_with_retry(
    client: &Client,
    url: &str,
) -> std::result::Result<reqwest::Response, reqwest::Error> {
    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3); // Retry up to 3 times
    let result = Retry::spawn(retry_strategy, || client.get(url).send()).await;
    result
}

#[allow(dead_code)]
struct HttpClient {
    client: Arc<Client>,
}

impl HttpClient {
    fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    #[allow(dead_code)]
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> std::result::Result<T, anyhow::Error> {
        let response = fetch_with_retry(&self.client, url).await?;
        Ok(response.json().await?)
    }
}

struct RequestBatcher {
    client: Arc<Client>,
    on_going_req:
        Arc<DashMap<String, Vec<oneshot::Sender<std::result::Result<Bytes, anyhow::Error>>>>>,
}

impl RequestBatcher {
    fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            on_going_req: Default::default(),
        }
    }

    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> std::result::Result<T, anyhow::Error> {
        let (tx, rx) = oneshot::channel();

        let should_fetch = {
            let mut entry = self.on_going_req.entry(url.to_owned()).or_default();
            entry.push(tx);
            entry.len() == 1
        };

        if should_fetch {
            let client = self.client.clone();
            let url = url.to_owned();
            let on_going_req = self.on_going_req.clone();

            tokio::spawn(async move {
                let result = fetch_with_retry(&client, &url).await;
                let senders = on_going_req.remove(&url);

                match result {
                    Ok(response) => match response.bytes().await {
                        Ok(body) => {
                            if let Some((_, senders)) = senders {
                                for sender in senders {
                                    let _ = sender.send(Ok(body.clone()));
                                }
                            }
                        }
                        Err(e) => {
                            if let Some((_, senders)) = senders {
                                for sender in senders {
                                    let err = anyhow::anyhow!(e.to_string());
                                    let _ = sender.send(Err(err));
                                }
                            }
                        }
                    },
                    Err(e) => {
                        if let Some((_, senders)) = senders {
                            for sender in senders {
                                let err = anyhow::anyhow!(e.to_string());
                                let _ = sender.send(Err(err));
                            }
                        }
                    }
                }
            });
        }

        let response = rx.await??;

        Ok(serde_json::from_slice(&response)?)
    }
}

fn create_schema() -> Schema<Query, EmptyMutation, EmptySubscription> {
    let client = Arc::new(Client::new());
    let http_client = Arc::new(HttpClient::new(client.clone()));
    let batch_client = Arc::new(RequestBatcher::new(client.clone()));

    let user_loader = DataLoader::new(UserLoader(batch_client.clone()), tokio::spawn)
        .delay(Duration::from_millis(1));
    let post_loader = DataLoader::new(PostLoader(batch_client.clone()), tokio::spawn)
        .delay(Duration::from_millis(5));

    Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(http_client)
        .data(user_loader)
        .data(post_loader)
        .data(batch_client)
        .data(Arc::new(Store::default()))
        .finish()
}

#[handler]
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().finish())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let schema = create_schema();

    // start the http server
    let app = Route::new()
        .at("/graphql", get(graphiql).post(GraphQL::new(schema.clone())))
        .at("/", get(graphiql).post(GraphQL::new(schema.clone())));
    Server::new(TcpListener::bind("0.0.0.0:8000"))
        .run(app)
        .await?;
    Ok(())
}
