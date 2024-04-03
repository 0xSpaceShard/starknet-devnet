use std::net::IpAddr;
use std::str::FromStr;

use hyper::header::HeaderValue;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub timeout: u16,
    pub request_body_size_limit: usize,
}

#[derive(Debug, Clone)]
pub struct HeaderValueWrapper(pub HeaderValue);

impl FromStr for HeaderValueWrapper {
    type Err = <HeaderValue as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HeaderValueWrapper(s.parse()?))
    }
}

impl Serialize for HeaderValueWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_str().map_err(serde::ser::Error::custom)?)
    }
}

impl<'de> Deserialize<'de> for HeaderValueWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s.parse().map_err(serde::de::Error::custom)?))
    }
}

impl std::ops::Deref for HeaderValueWrapper {
    type Target = HeaderValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<HeaderValueWrapper> for HeaderValue {
    fn from(wrapper: HeaderValueWrapper) -> Self {
        wrapper.0
    }
}

impl From<HeaderValue> for HeaderValueWrapper {
    fn from(header: HeaderValue) -> Self {
        HeaderValueWrapper(header)
    }
}
