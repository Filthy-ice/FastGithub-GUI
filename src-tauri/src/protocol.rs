use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum Request {
    Enable { kernel_dir: String },
    Disable,
    Status,
    Uninstall,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Response {
    pub ok: bool,
    pub state: String,
    pub message: String,
}

impl Response {
    pub fn ok(state: &str) -> Self { Response { ok: true, state: state.into(), message: String::new() } }
    pub fn err(state: &str, msg: impl Into<String>) -> Self { Response { ok: false, state: state.into(), message: msg.into() } }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enable_serialize() {
        let r = Request::Enable { kernel_dir: "/x".into() };
        assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"cmd":"enable","kernel_dir":"/x"}"#);
    }
    #[test]
    fn disable_roundtrip() {
        let s = serde_json::to_string(&Request::Disable).unwrap();
        assert_eq!(s, r#"{"cmd":"disable"}"#);
        assert_eq!(serde_json::from_str::<Request>(&s).unwrap(), Request::Disable);
    }
    #[test]
    fn response_helpers() {
        assert!(Response::ok("on").ok);
        assert!(!Response::err("off", "boom").ok);
    }
}