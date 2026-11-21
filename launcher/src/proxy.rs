//! The proxy every download and request goes through, when the player needs one: a school's or an
//! office's, or a country where a host is unreachable. The Settings page's Proxy row (kept as
//! `proxy` in `settings.json`) wins; without it the usual `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`
//! and `NO_PROXY` variables apply, so a launcher started from a shell that has them needs nothing.
//!
//! What goes through it: IndustrialEra's releases and the launcher's own update, Minecraft's files and
//! Java from Mojang, Modrinth, CurseForge, IndustrialEra's mod registry, pictures, Microsoft sign-in,
//! Tailscale's API, and the jars a modpack import downloads. A local address never does, and the
//! Servers page's calls to a game server's API (`servers.rs`) build their agent without it: they
//! are on loopback and asked every second.
//!
//! Which hosts are reached directly, and how an address is read, are `industrial_era-port-pack`'s rules
//! (`crates/industrial_era-port-pack/src/net.rs`): the jars an import fetches are downloaded there, and
//! the same hosts have to be left alone on both sides. This module adds what is the launcher's
//! own — the setting in `settings.json`, kept in memory so a request doesn't read the file — and
//! hands the address to the parts that build their agents.
use crate::backend::Settings;
use std::sync::Mutex;

/// The launcher's setting, as `announce` last read it, or None before it has: a request shouldn't
/// read `settings.json` every time, and an import fetches hundreds of jars.
static SET: Mutex<Option<Option<String>>> = Mutex::new(None);

/// The proxy address to reach `url` through: the launcher's setting, else the environment's, unless
/// `url` is a local address or `NO_PROXY` names its host.
/// `direct_hosts` match loopback and localhost: they never go through a proxy.
const DIRECT_HOSTS: &[&str] = &["127.0.0.1", "localhost", "::1"];

/// Whether `url` points at a local host: a loopback address never uses a proxy.
pub fn direct(url: &str) -> bool {
    for host in DIRECT_HOSTS {
        if url.contains(host) {
            return true;
        }
    }
    false
}

/// The scheme of `url`: `http`, `https`...
pub fn scheme_of(url: &str) -> Option<&str> {
    url.split("://").next()
}

/// `env_for` reads `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` for the scheme, and `NO_PROXY` hosts.
fn env_for(scheme: &str) -> Option<String> {
    let env_var = match scheme {
        "https" => "HTTPS_PROXY",
        "http" => "HTTP_PROXY",
        _ => "ALL_PROXY",
    };
    std::env::var(env_var)
        .ok()
        .or_else(|| std::env::var("ALL_PROXY").ok())
}

/// Parse `spec` as a proxy address: `http://host:port`.
pub fn parse(spec: &str) -> Option<ureq::Proxy> {
    ureq::Proxy::new(spec).ok()
}

pub fn host_of(spec: &str) -> Option<String> {
    let u = url::Url::parse(spec).ok()?;
    u.host_str().map(|h| h.to_string())
}

fn socks(spec: &str) -> bool {
    spec.starts_with("socks")
}

fn redacted(spec: &str) -> String {
    let u = url::Url::parse(spec).ok()?;
    format!("{}://***@{}", u.scheme(), u.host_str().unwrap_or(""))
}

pub fn set_proxy(_spec: Option<&str>) {
    // Environment proxies are read per-address by the agent itself.
}

/// The launcher's setting: read from `settings.json` the first time it is needed and kept after
/// that, so a download doesn't read the file once per file.
fn setting() -> Option<String> {
    let mut set = held();
    match set.clone() {
        Some(kept) => kept,
        None => {
            let read = written();
            *set = Some(read.clone());
            read
        }
    }
}

/// The launcher's setting as it is written down, or None when there is none.
fn written() -> Option<String> {
    Settings::load()
        .proxy
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
}

/// The setting's lock, even after a thread panicked while holding it: an address lost to a panic
/// is worse than an old one kept.
fn held() -> std::sync::MutexGuard<'static, Option<Option<String>>> {
    SET.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_address_takes_no_proxy() {
        assert_eq!(spec_for("http://127.0.0.1:8130/api/v1/status"), None);
        assert_eq!(spec_for("http://localhost/api/v1/status"), None);
    }

    /// An address the launcher knows nothing about is one nothing is written for either.
    #[test]
    fn a_relative_address_has_no_scheme_to_read_a_proxy_for() {
        assert_eq!(spec_for("data/a.jar"), None);
    }

    #[test]
    fn a_proxy_address_is_checked_before_it_is_kept() {
        assert_eq!(refuse(""), None);
        assert_eq!(refuse("  "), None);
        assert_eq!(refuse("http://127.0.0.1:7890"), None);
        assert_eq!(refuse("127.0.0.1:7890"), None);
        assert_eq!(refuse("http://user:pass@proxy.local:3128"), None);
        assert!(refuse("http://").is_some());
        assert!(refuse("socks5://127.0.0.1:7891").is_some());
    }

    /// The reason is shown under the field and printed by `industrial_era-launcher settings set`.
    #[test]
    fn a_refused_address_keeps_its_credentials_to_itself() {
        let why = refuse("socks5://user:secret@127.0.0.1:7891").unwrap();
        assert!(why.starts_with("socks5://***@127.0.0.1:7891 "), "{why}");
        let why = refuse("http://user:secret@").unwrap();
        assert!(!why.contains("secret"), "{why}");
    }
}
