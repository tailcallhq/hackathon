use crate::endpoint::Endpoint;
use crate::hasher::MyHasher;
use crate::http::query_encoder::QueryEncoder;
use crate::ir::eval_ctx::EvalContext;
use crate::ir::IoId;
use crate::mustache::model::{Mustache, Segment};
use crate::path::{PathString, PathValue, ValueString};
use crate::value::Value;
use hyper::Method;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use url::Url;

#[derive(Debug, Clone)]
pub struct RequestTemplate {
    pub root_url: Mustache,
    pub query: Vec<Query>,
    pub method: Method,
    pub query_encoder: QueryEncoder,
    // pub headers: MustacheHeaders,
}

#[derive(Debug, Clone)]
pub struct Query {
    pub key: String,
    pub value: Mustache,
    pub skip_empty: bool,
}

impl TryFrom<Endpoint> for RequestTemplate {
    type Error = anyhow::Error;
    fn try_from(endpoint: Endpoint) -> anyhow::Result<Self> {
        let path = Mustache::parse(endpoint.path.as_str());
        let query = endpoint
            .query
            .iter()
            .map(|(k, v, skip)| {
                Ok(Query {
                    key: k.as_str().to_string(),
                    value: Mustache::parse(v.as_str()),
                    skip_empty: *skip,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let method = endpoint.method.clone().to_hyper();
        /*        let headers = endpoint
        .headers
        .iter()
        .map(|(k, v)| Ok((k.clone(), Mustache::parse(v.to_str()?))))
        .collect::<anyhow::Result<Vec<_>>>()?;*/

        Ok(Self {
            root_url: path,
            query,
            method,
            // headers,
            query_encoder: Default::default(),
        })
    }
}

impl RequestTemplate {
    pub fn cache_key(&self, ctx: &EvalContext) -> IoId {
        let mut hasher = MyHasher::default();
        let state = &mut hasher;

        self.method.hash(state);

        /* for (name, value) in ctx.headers().iter() {
            name.hash(state);
            value.hash(state);
        }*/

        let url = self.create_url(ctx).unwrap();
        url.hash(state);

        IoId::new(hasher.finish())
    }
}

struct ValueStringEval<A>(std::marker::PhantomData<A>);
impl<A> Default for ValueStringEval<A> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<'a, A: PathValue> ValueStringEval<A> {
    fn eval(&self, mustache: &Mustache, in_value: &'a A) -> Option<ValueString<'a>> {
        mustache
            .segments()
            .iter()
            .filter_map(|segment| match segment {
                Segment::Literal(text) => Some(ValueString::Value(Cow::Owned(Value::new(
                    serde_json::Value::String(text.to_owned()),
                )))),
                Segment::Expression(parts) => in_value.raw_value(parts),
            })
            .next() // Return the first value that is found
    }
}

impl RequestTemplate {
    /// Creates a URL for the context
    /// Fills in all the mustache templates with required values.
    fn create_url<C: PathString + PathValue>(&self, ctx: &C) -> anyhow::Result<Url> {
        let mut url = url::Url::parse(self.root_url.render(ctx).as_str())?;
        if self.query.is_empty() && self.root_url.is_const() {
            return Ok(url);
        }

        // evaluates mustache template and returns the values evaluated by mustache
        // template.
        let mustache_eval = ValueStringEval::default();

        let extra_qp = self.query.iter().filter_map(|query| {
            let key = &query.key;
            let value = &query.value;
            let skip = query.skip_empty;
            let parsed_value = mustache_eval.eval(value, ctx);
            if skip && parsed_value.is_none() {
                None
            } else {
                Some(self.query_encoder.encode(key, parsed_value))
            }
        });

        let base_qp = url
            .query_pairs()
            .filter_map(|(k, v)| if v.is_empty() { None } else { Some((k, v)) });

        let qp_string = base_qp.map(|(k, v)| format!("{}={}", k, v));
        let qp_string = qp_string.chain(extra_qp).fold("".to_string(), |str, item| {
            if str.is_empty() {
                item
            } else if item.is_empty() {
                str
            } else {
                format!("{}&{}", str, item)
            }
        });

        if qp_string.is_empty() {
            url.set_query(None);
            Ok(url)
        } else {
            url.set_query(Some(qp_string.as_str()));
            Ok(url)
        }
    }

    /// Checks if the template has any mustache templates or not
    /// Returns true if there are not templates
    pub fn is_const(&self) -> bool {
        self.root_url.is_const() && self.query.iter().all(|query| query.value.is_const())
    }

    /// Creates a Request for the given context
    pub fn to_request<C: PathString + PathValue>(
        &self,
        ctx: &C,
    ) -> anyhow::Result<reqwest::Request> {
        // Create url
        let url = self.create_url(ctx)?;
        let method = self.method.clone();
        let req = reqwest::Request::new(
            crate::http::method::Method::from(method).into_reqwest(),
            url,
        );
        // req = self.set_headers(req, ctx);
        // req = self.set_body(req, ctx)?;

        Ok(req)
    }

    /*    /// Sets the headers for the request
    fn set_headers<C: PathString>(
        &self,
        mut req: reqwest::Request,
        ctx: &C,
    ) -> reqwest::Request {
        let headers = self.create_headers(ctx);
        if !headers.is_empty() {
            req.headers_mut().extend(headers);
        }

        let headers = req.headers_mut();
        // We want to set the header value based on encoding
        // TODO: potential of optimizations.
        // Can set content-type headers while creating the request template
        if self.method != reqwest::Method::GET {
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
        }

        headers.extend(ctx.headers().to_owned());
        req
    }*/

    pub fn new(root_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            root_url: Mustache::parse(root_url),
            query: Default::default(),
            method: Method::GET,
            query_encoder: Default::default(),
        })
    }
}
