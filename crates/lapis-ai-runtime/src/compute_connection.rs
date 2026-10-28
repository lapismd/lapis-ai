use crate::{Error, Result};

/// Native host-only credential, intentionally neither serializable nor Debug-visible.
#[derive(Clone)]
pub struct ComputeConnection {
    pub(crate) endpoint: reqwest::Url,
    pub(crate) token: zeroize::Zeroizing<String>,
}
impl std::fmt::Debug for ComputeConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputeConnection").finish_non_exhaustive()
    }
}
impl ComputeConnection {
    pub fn endpoint(&self) -> &reqwest::Url {
        &self.endpoint
    }
    pub fn bearer_token(&self) -> &str {
        &self.token
    }
    pub fn peer_endpoint(&self, identity: &str) -> Result<reqwest::Url> {
        if identity.len() != 64 || !identity.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error("invalid_peer_identity"));
        }
        let mut endpoint = self.endpoint.clone();
        endpoint.set_path(&format!("/peer/{identity}/v1"));
        Ok(endpoint)
    }
}
