use crate::http::headers::HeaderMap;
use anyhow::Result;
use async_graphql_value::{ConstValue, Name};
use derive_setters::Setters;
use http::StatusCode;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;

#[derive(Clone, Debug, Default, Setters)]
pub struct Response<Body> {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
}

// Trait to convert a serde_json_borrow::Value to a ConstValue.
// serde_json_borrow::Value is a borrowed version of serde_json::Value.
// It has a limited lifetime tied to the input JSON, making it more
// efficient. Benchmarking is required to determine the performance If any
// change is made.

pub trait FromValue {
    fn from_value(value: serde_json_borrow::Value) -> Self;
}

impl FromValue for ConstValue {
    fn from_value(value: serde_json_borrow::Value) -> Self {
        match value {
            serde_json_borrow::Value::Null => ConstValue::Null,
            serde_json_borrow::Value::Bool(b) => ConstValue::Boolean(b),
            serde_json_borrow::Value::Number(n) => ConstValue::Number(n.into()),
            serde_json_borrow::Value::Str(s) => ConstValue::String(s.into()),
            serde_json_borrow::Value::Array(a) => {
                ConstValue::List(a.into_iter().map(|v| Self::from_value(v)).collect())
            }
            serde_json_borrow::Value::Object(o) => ConstValue::Object(
                o.into_vec()
                    .into_iter()
                    .map(|(k, v)| (Name::new(k), Self::from_value(v)))
                    .collect(),
            ),
        }
    }
}

impl Response<Bytes> {
    pub async fn from_reqwest(resp: reqwest::Response) -> Result<Self> {
        let status = StatusCode::from_u16(resp.status().as_u16())?;
        let headers = HeaderMap::from(resp.headers().to_owned());
        let body = resp.bytes().await?;
        Ok(Response {
            status,
            headers,
            body,
        })
    }

    pub async fn from_hyper(resp: hyper::Response<Full<Bytes>>) -> Result<Self> {
        let status = resp.status();
        let headers = HeaderMap::from(resp.headers().to_owned());
        let body = resp.into_body().collect().await?.to_bytes();
        Ok(Response {
            status,
            headers,
            body,
        })
    }

    pub fn empty() -> Self {
        Response {
            status: StatusCode::OK,
            headers: HeaderMap::default(),
            body: Bytes::new(),
        }
    }
    pub fn to_serde_json(self) -> Result<Response<serde_json::Value>> {
        if self.body.is_empty() {
            return Ok(Response {
                status: self.status,
                headers: self.headers,
                body: serde_json::Value::Null,
            });
        }
        let body: serde_json::Value = serde_json::from_slice(&self.body)?;
        Ok(Response {
            status: self.status,
            headers: self.headers,
            body,
        })
    }

    pub fn to_json<T: Default + FromValue>(self) -> Result<Response<T>> {
        if self.body.is_empty() {
            return Ok(Response {
                status: self.status,
                headers: self.headers,
                body: Default::default(),
            });
        }
        // Note: We convert the body to a serde_json_borrow::Value for better
        // performance. Warning: Do not change this to direct conversion to `T`
        // without benchmarking the performance impact.
        let body: serde_json_borrow::Value = serde_json::from_slice(&self.body)?;
        let body = T::from_value(body);
        Ok(Response {
            status: self.status,
            headers: self.headers,
            body,
        })
    }

    pub fn to_resp_string(self) -> Result<Response<String>> {
        Ok(Response::<String> {
            body: String::from_utf8(self.body.to_vec())?,
            status: self.status,
            headers: self.headers,
        })
    }
}

impl From<Response<Bytes>> for hyper::Response<Full<Bytes>> {
    fn from(resp: Response<Bytes>) -> Self {
        let mut response = hyper::Response::new(Full::new(resp.body));
        *response.headers_mut() = resp.headers.into();
        *response.status_mut() = resp.status;
        response
    }
}
