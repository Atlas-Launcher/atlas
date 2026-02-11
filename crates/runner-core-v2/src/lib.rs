mod errors;
pub mod proto;

pub const PROTOCOL_VERSION: u32 = 1;

pub fn pkg_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
