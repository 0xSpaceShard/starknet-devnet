use std::fmt::Display;
use std::net::{AddrParseError, IpAddr, Ipv4Addr};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IpAddrWrapper {
    pub(crate) inner: IpAddr,
}

impl FromStr for IpAddrWrapper {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "localhost" => Ok(IpAddrWrapper::LOCALHOST),
            other => Ok(IpAddrWrapper { inner: IpAddr::V4(Ipv4Addr::from_str(other)?) }),
        }
    }
}

impl Display for IpAddrWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl Default for IpAddrWrapper {
    fn default() -> Self {
        Self::LOCALHOST
    }
}

impl IpAddrWrapper {
    pub(crate) const LOCALHOST: Self = IpAddrWrapper { inner: IpAddr::V4(Ipv4Addr::LOCALHOST) };
}
