use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Error(pub &'static str);
pub type Result<T> = std::result::Result<T, Error>;
impl From<lapis_ai_runtime::Error> for Error {
    fn from(error: lapis_ai_runtime::Error) -> Self {
        Self(error.0)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
