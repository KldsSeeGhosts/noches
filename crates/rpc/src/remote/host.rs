//! Host-side remote access: tailnet discovery, pairing, and revocation.
//!
//! The CLI (`noches-connect`) and the desktop Settings > Connections page share
//! this module so both mint, validate, and revoke pairing codes identically.

use super::{ConnectionProfile, Credentials, private_write};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
    time::Duration,
};

/// The gateway's well-known port. Matches `docs/tailnet-connections.md`.
pub const GATEWAY_PORT: u16 = 27657;

/// Tailscale hands every node an address in the CGNAT range `100.64.0.0/10`.
fn is_tailnet(ip: Ipv4Addr) -> bool {
    // Same predicate the endpoint validator uses for `private_ip`.
    u32::from(ip) & 0xffc00000 == 0x64400000
}

/// First Tailscale address in a list, in interface order.
///
/// Pure so the selection rule is unit-testable without a live tailnet. Feeds
/// [`local_addresses`].
pub fn select_tailnet(addrs: &[IpAddr]) -> Option<Ipv4Addr> {
    addrs.iter().find_map(|addr| match addr {
        IpAddr::V4(ip) if is_tailnet(*ip) => Some(*ip),
        _ => None,
    })
}

#[cfg(unix)]
fn local_addresses() -> Vec<IpAddr> {
    let mut result = Vec::new();
    // SAFETY: `getifaddrs` fills a caller-owned list on success; every pointer
    // below is checked before use and the list is released exactly once, after
    // the walk.
    unsafe {
        let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut head) != 0 {
            return result;
        }
        let mut cursor = head;
        while !cursor.is_null() {
            let entry = &*cursor;
            let family = if entry.ifa_addr.is_null() {
                -1
            } else {
                (*entry.ifa_addr).sa_family as i32
            };
            if family == libc::AF_INET {
                let addr = &*(entry.ifa_addr as *const libc::sockaddr_in);
                // `sin_addr.s_addr` is stored in network byte order.
                result.push(IpAddr::V4(Ipv4Addr::from(u32::from_be(
                    addr.sin_addr.s_addr,
                ))));
            }
            cursor = entry.ifa_next;
        }
        libc::freeifaddrs(head);
    }
    result
}

#[cfg(not(unix))]
fn local_addresses() -> Vec<IpAddr> {
    Vec::new()
}

/// The computer's Tailscale IPv4 address, or `None` when it is off the tailnet.
pub fn tailnet_ipv4() -> Option<Ipv4Addr> {
    select_tailnet(&local_addresses())
}

fn mint_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Pair a phone with the engine behind `upstream` and return its code.
///
/// Reads the engine's `deviceId`, mints a 64-hex key, appends the client to
/// `credentials`, and saves it with mode 0600. The caller owns the returned
/// code — it is never logged and never written anywhere but `credentials`.
pub async fn pair_client(
    upstream: &str,
    credentials: &Path,
    name: &str,
    endpoint: &str,
) -> anyhow::Result<ConnectionProfile> {
    let engine = crate::connect_ws(upstream).await?;
    let info = tokio::time::timeout(
        Duration::from_secs(8),
        engine.call(crate::methods::ENGINE_INFO, serde_json::json!({})),
    )
    .await??;
    let profile = ConnectionProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        endpoint: endpoint.to_string(),
        token: mint_token(),
        device_id: info["deviceId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing engine identity"))?
            .into(),
    };
    // Validate before the append, so a bad endpoint never reaches disk.
    profile.validate()?;
    let mut store = Credentials::load(credentials)?;
    store.clients.push(profile.clone());
    store.save(credentials)?;
    Ok(profile)
}

/// Remove one client by id. A live gateway re-reads the credentials file on its
/// five-second tick, so that client's sessions close within five seconds.
pub fn revoke_client(credentials: &Path, id: &str) -> anyhow::Result<()> {
    let mut store = Credentials::load(credentials)?;
    let before = store.clients.len();
    store.clients.retain(|c| c.id != id);
    anyhow::ensure!(store.clients.len() < before, "Client not found");
    store.save(credentials)
}

/// Write a pairing code to a private file the way the CLI's `pair` does.
pub fn write_code(path: &Path, code: &str) -> anyhow::Result<()> {
    private_write(path, code.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RpcError, RpcReply, RpcService};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct EngineInfoRpc;

    #[async_trait]
    impl RpcService for EngineInfoRpc {
        async fn handle(
            &self,
            method: &str,
            _params: serde_json::Value,
        ) -> Result<RpcReply, RpcError> {
            match method {
                crate::methods::ENGINE_INFO => {
                    RpcReply::value(&serde_json::json!({"deviceId":"test-mac"}))
                }
                other => Err(RpcError::UnknownMethod(other.into())),
            }
        }
    }

    #[test]
    fn tailnet_selection_ignores_everything_but_cgnat_v4() {
        let cases = [
            (vec![], None),
            (vec!["127.0.0.1".parse().unwrap()], None),
            (vec!["192.168.1.20".parse().unwrap()], None),
            (vec!["fd7a:115c:a1e0::1".parse().unwrap()], None),
            // The /10 boundary: 100.63.255.255 is below, 100.64.0.0 inside,
            // 100.127.255.255 inside, 100.128.0.0 above.
            (vec!["100.63.255.255".parse().unwrap()], None),
            (
                vec!["100.64.0.0".parse().unwrap()],
                Some("100.64.0.0".parse().unwrap()),
            ),
            (
                vec!["100.127.255.255".parse().unwrap()],
                Some("100.127.255.255".parse().unwrap()),
            ),
            (vec!["100.128.0.0".parse().unwrap()], None),
            // First tailnet address wins over later ones and over non-tailnet
            // interfaces that enumerated before it.
            (
                vec![
                    "192.168.1.20".parse().unwrap(),
                    "100.114.177.75".parse().unwrap(),
                    "100.64.0.9".parse().unwrap(),
                ],
                Some("100.114.177.75".parse().unwrap()),
            ),
            (
                vec![
                    "fe80::1".parse().unwrap(),
                    "100.101.102.103".parse().unwrap(),
                ],
                Some("100.101.102.103".parse().unwrap()),
            ),
        ];
        for (addrs, expected) in cases {
            assert_eq!(select_tailnet(&addrs), expected, "{addrs:?}");
        }
    }

    #[test]
    fn local_addresses_are_well_formed() {
        // Whatever this machine has, the enumeration must not crash and every
        // entry has to be a real unicast-or-better address.
        for addr in local_addresses() {
            assert!(addr.is_ipv4(), "{addr}");
        }
        if let Some(ip) = tailnet_ipv4() {
            assert!(is_tailnet(ip), "{ip}");
        }
    }

    #[tokio::test]
    async fn pair_client_registers_a_valid_code_and_revoke_removes_it() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(crate::serve_ws_listener(listener, Arc::new(EngineInfoRpc)));
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("access.json");
        let upstream = format!("ws://127.0.0.1:{port}");

        let profile = pair_client(&upstream, &store, "Studio Mac", "ws://100.64.0.1:27657")
            .await
            .unwrap();
        assert_eq!(profile.device_id, "test-mac");
        assert_eq!(profile.name, "Studio Mac");
        assert_eq!(profile.token.len(), 64);
        assert!(profile.token.bytes().all(|b| b.is_ascii_hexdigit()));
        // The published code round-trips and the file is 0600.
        let code = profile.code().unwrap();
        assert_eq!(
            ConnectionProfile::from_code(&code).unwrap().token,
            profile.token
        );
        let saved = Credentials::load(&store).unwrap();
        assert_eq!(saved.clients.len(), 1);
        assert!(saved.authorizes(&profile.token));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&store).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        // A second pairing appends rather than replacing.
        let second = pair_client(&upstream, &store, "Phone rig", "ws://100.64.0.1:27657")
            .await
            .unwrap();
        assert_eq!(Credentials::load(&store).unwrap().clients.len(), 2);
        assert_ne!(second.token, profile.token);

        revoke_client(&store, &profile.id).unwrap();
        let after = Credentials::load(&store).unwrap();
        assert_eq!(after.clients.len(), 1);
        assert!(!after.authorizes(&profile.token));
        // Revoking an unknown id is an error, not a silent no-op.
        assert!(revoke_client(&store, &profile.id).is_err());
        // The revoked key never comes back through the remaining client.
        assert_eq!(after.clients[0].id, second.id);
    }

    #[tokio::test]
    async fn pair_client_rejects_an_endpoint_that_is_not_private() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(crate::serve_ws_listener(listener, Arc::new(EngineInfoRpc)));
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("access.json");
        let result = pair_client(
            &format!("ws://127.0.0.1:{port}"),
            &store,
            "Studio Mac",
            "ws://example.com:27657",
        )
        .await;
        assert!(result.is_err());
        // Nothing was written for a rejected profile.
        assert!(!store.exists());
    }

    #[tokio::test]
    async fn pair_client_surfaces_a_missing_engine_identity() {
        struct NamelessRpc;
        #[async_trait]
        impl RpcService for NamelessRpc {
            async fn handle(
                &self,
                method: &str,
                _params: serde_json::Value,
            ) -> Result<RpcReply, RpcError> {
                match method {
                    crate::methods::ENGINE_INFO => RpcReply::value(&serde_json::json!({})),
                    other => Err(RpcError::UnknownMethod(other.into())),
                }
            }
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(crate::serve_ws_listener(listener, Arc::new(NamelessRpc)));
        let dir = tempfile::tempdir().unwrap();
        let error = pair_client(
            &format!("ws://127.0.0.1:{port}"),
            &dir.path().join("access.json"),
            "Studio Mac",
            "ws://100.64.0.1:27657",
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("engine identity"), "{error}");
    }
}
