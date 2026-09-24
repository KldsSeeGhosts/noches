//! Keep transport causes when HTTP errors cross string-based RPC/sync seams.

use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Whether `url` targets a destination that must be reached DIRECTLY, without
/// consulting any configured proxy. Shared by every HTTP client below that can
/// be pointed at a private edge. reqwest's `system-proxy` support has no
/// built-in private-address exemption and cannot be relied on to apply the OS
/// bypass list, so without this check a bearer token or exchange code meant
/// for a local or tailnet edge could be handed to the machine-wide proxy
/// instead of going to the destination. `NO_PROXY` still applies on top.
///
/// Direct destinations:
/// - loopback (`localhost`, `*.localhost`, 127.0.0.0/8, ::1, IPv4-mapped
///   ::ffff:127.x)
/// - unspecified bind addresses (0.0.0.0, ::) - a client dialing them is
///   aiming at this machine
/// - private IPv4: RFC1918 (10/8, 172.16/12, 192.168/16)
/// - link-local (169.254/16, fe80::/10)
/// - CGNAT, which is also the Tailscale IPv4 range (100.64.0.0/10)
/// - IPv6 ULA (fc00::/7), which covers Tailscale's fd7a:115c:a1e0::/48
/// - `*.ts.net` and `*.local` hostnames (trailing dot and case tolerant)
pub(crate) fn is_direct_destination(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    // IPv6 literals serialize with brackets (`[::1]`); strip them so the
    // address parses. Hostnames are already lowercased by URL parsing; a
    // trailing root dot is normalized off for suffix matching.
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_direct_ip(ip);
    }
    let name = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
    if name.is_empty() {
        return false;
    }
    // Other single-label names stay on the proxy: a corporate edge such as
    // `https://edge/` may only be reachable through it. Tailnet device windows
    // use the WebSocket transport in `zeron-rpc`, which never reads the proxy
    // environment; anything else can opt out through `NO_PROXY`.
    name == "localhost"
        || name.ends_with(".localhost")
        || name.ends_with(".ts.net")
        || name.ends_with(".local")
}

/// Whether `ip` is a private/tailnet address that a system proxy should never
/// carry traffic for. See [`is_direct_destination`].
fn is_direct_ip(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(ip) => is_direct_ipv4(ip),
        IpAddr::V6(ip) => is_direct_ipv6(ip),
    }
}

fn is_direct_ipv4(ip: Ipv4Addr) -> bool {
    // to_canonical() unwraps IPv4-mapped ::ffff:x, so a mapped loopback or
    // private address lands here. `is_shared` (100.64.0.0/10) is still
    // unstable, so the CGNAT check is an explicit octet comparison.
    let [a, b, ..] = ip.octets();
    ip.is_loopback()                 // 127.0.0.0/8
        || ip.is_unspecified()       // 0.0.0.0
        || ip.is_private()           // RFC1918: 10/8, 172.16/12, 192.168/16
        || ip.is_link_local()        // 169.254.0.0/16
        || (a == 100 && (64..=127).contains(&b)) // CGNAT / Tailscale IPv4
}

fn is_direct_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()                                  // ::1
        || ip.is_unspecified()                        // ::
        || (ip.segments()[0] & 0xffc0) == 0xfe80      // link-local fe80::/10
        // ULA fc00::/7, which includes Tailscale's fd7a:115c:a1e0::/48.
        || (ip.segments()[0] & 0xfe00) == 0xfc00
}

/// A `reqwest::ClientBuilder` that honors the system proxy except for
/// destinations in [`is_direct_destination`]. reqwest's `system-proxy` feature
/// has no loopback/private exemption and does not consult the OS bypass list,
/// so without this a bearer token meant for a local or tailnet edge would be
/// sent through the proxy.
pub(crate) fn client_builder_for(target_url: &str) -> reqwest::ClientBuilder {
    apply_proxy_policy(reqwest::Client::builder(), target_url)
}

/// Clear every proxy (system or explicit) from `builder` when `target_url` is
/// a direct destination; leave it untouched otherwise.
fn apply_proxy_policy(builder: reqwest::ClientBuilder, target_url: &str) -> reqwest::ClientBuilder {
    if is_direct_destination(target_url) {
        builder.no_proxy()
    } else {
        builder
    }
}

/// A plain client for `target_url`, direct when the target is a private or
/// local destination.
pub(crate) fn client_for(target_url: &str) -> reqwest::Client {
    client_builder_for(target_url)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// reqwest's Display omits its source chain, including DNS/TLS/socket errors.
/// Retain that chain and the destination origin, excluding URL credentials,
/// paths and queries that may contain tokens or other private values.
pub(crate) fn describe_http_error(error: reqwest::Error) -> String {
    let origin = error.url().map(|url| url.origin().ascii_serialization());
    let error = error.without_url();
    let mut message = match origin {
        Some(origin) => format!("{origin}: {error}"),
        None => error.to_string(),
    };
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    pub(crate) struct FailingDns {
        pub(crate) calls: AtomicUsize,
    }

    impl reqwest::dns::Resolve for FailingDns {
        fn resolve(&self, _: reqwest::dns::Name) -> reqwest::dns::Resolving {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(std::io::Error::other("injected DNS lookup failure").into()) })
        }
    }

    impl FailingDns {
        pub(crate) fn client(self: &Arc<Self>) -> reqwest::Client {
            reqwest::Client::builder()
                .no_proxy()
                .dns_resolver(self.clone())
                .build()
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_destination_policy() {
        for url in [
            // loopback hostnames
            "http://localhost:8787/registry/x/rows",
            "http://LOCALHOST:8787/",
            "http://localhost.:8787/",
            "https://api.localhost/health",
            "https://app.localhost./health",
            // loopback IPs (incl. shorthand and mapped forms the URL parser
            // normalizes)
            "http://127.0.0.1:8787/registry/x/rows",
            "http://127.255.0.1/",
            "http://127.1/",
            "http://0x7f000001/",
            "http://[::1]:8787/health",
            "http://[::ffff:127.0.0.1]/",
            // unspecified
            "http://0.0.0.0:8080/",
            "http://[::]:8080/",
            // RFC1918
            "http://10.0.0.5/",
            "http://10.255.255.255/",
            "http://172.16.0.1/",
            "http://172.31.255.255/",
            "http://192.168.1.5/health",
            // link-local
            "http://169.254.1.1/",
            "http://[fe80::1]/",
            "http://[fe80::dead:beef]/",
            // CGNAT / Tailscale IPv4
            "http://100.64.0.1:27657/",
            "http://100.114.177.75:27657/",
            "http://100.127.255.255/",
            // IPv6 ULA (covers Tailscale fd7a:115c:a1e0::/48)
            "http://[fd7a:115c:a1e0::1]:27657/",
            "http://[fc00::1]/",
            "http://[fdff::ffff]/",
            // ts.net / local hostnames (case + trailing dot tolerant)
            "https://host.tail-scale.ts.net/",
            "https://HOST.TS.NET./",
            "https://printer.local/",
            "https://printer.LOCAL./",
        ] {
            assert!(is_direct_destination(url), "expected direct: {url}");
        }
        for url in [
            // public IPs
            "https://edge.example.com/health",
            "https://8.8.8.8/",
            "https://1.1.1.1/",
            // just outside the ranges we bypass
            "http://100.63.255.255/",
            "http://100.128.0.0/",
            "http://172.15.0.1/",
            "http://172.32.0.1/",
            "http://169.253.1.1/",
            "http://[fe7f::1]/",
            "http://[fbff::1]/",
            // single-label names other than localhost (e.g. a corporate edge)
            "https://edge/",
            "http://nas./",
            // look-alike public hostnames
            "https://localhost.evil.com/",
            "https://evil-localhost.com/",
            "https://ts.net.evil.com/",
            "https://evilts.net/",
            "https://local.evil.com/",
            "https://notlocal.com/",
            "https://edge.ts.net.evil.com./",
            // not a URL / no host
            "not a url",
            "",
            "file:///etc/hosts",
        ] {
            assert!(!is_direct_destination(url), "expected proxied: {url}");
        }
    }

    /// A credential-bearing request to a loopback edge must go DIRECT even
    /// when a proxy is configured. The proxy is injected explicitly rather
    /// than through HTTP(S)_PROXY: `no_proxy()` clears system and explicit
    /// proxies alike, and mutating the process environment would race every
    /// other test in this binary that builds a client. The recording proxy
    /// accepts the TCP connection then drops it; the stub edge answers only on
    /// a direct dial.
    #[tokio::test]
    async fn loopback_edge_bypasses_configured_proxy() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Recording proxy on loopback: accepts and drops, counting hits.
        let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_hits = std::sync::Arc::new(AtomicUsize::new(0));
        let hits = proxy_hits.clone();
        let proxy_task = tokio::spawn(async move {
            while let Ok((mut socket, _)) = proxy_listener.accept().await {
                hits.fetch_add(1, Ordering::SeqCst);
                let _ = socket.shutdown().await;
            }
        });

        // Stub edge on loopback answering a GET.
        let edge_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let edge_addr = edge_listener.local_addr().unwrap();
        let edge_task = tokio::spawn(async move {
            let (mut socket, _) = edge_listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok")
                .await
                .unwrap();
        });

        let with_proxy = || {
            reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(format!("http://{proxy_addr}")).unwrap())
        };
        let direct = apply_proxy_policy(with_proxy(), &format!("http://{edge_addr}/"))
            .build()
            .unwrap();
        let proxied = apply_proxy_policy(with_proxy(), "https://edge.example.com/")
            .build()
            .unwrap();

        // The loopback edge is reached directly: the proxy sees nothing.
        let body = direct
            .get(format!("http://{edge_addr}/health"))
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(&body[..], b"ok");
        edge_task.await.unwrap();
        assert_eq!(proxy_hits.load(Ordering::SeqCst), 0);

        // A public destination still goes through the configured proxy: the
        // CONNECT (TLS target) hits the recorder, which drops it.
        let proxied_result = proxied.get("https://edge.example.com/health").send().await;
        assert!(proxied_result.is_err());
        assert_eq!(proxy_hits.load(Ordering::SeqCst), 1);
        proxy_task.abort();
    }

    #[tokio::test]
    async fn reports_dns_cause_without_url_secrets() {
        let dns = std::sync::Arc::new(test_support::FailingDns::default());
        let error = dns.client()
            .get("https://login-user:password-secret@edge.invalid/private-path?token=query-secret#fragment-secret")
            .send().await.unwrap_err();
        assert!(!error.to_string().contains("injected DNS lookup failure"));

        let message = describe_http_error(error);
        assert!(message.contains("https://edge.invalid"), "{message}");
        assert!(message.contains("injected DNS lookup failure"), "{message}");
        for secret in [
            "login-user",
            "password-secret",
            "private-path",
            "query-secret",
            "fragment-secret",
        ] {
            assert!(!message.contains(secret), "URL secret leaked: {message}");
        }
    }
}
