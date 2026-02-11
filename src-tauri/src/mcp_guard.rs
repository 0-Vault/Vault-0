use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

static ALLOWED_ORIGINS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("localhost".to_string());
    s.insert("127.0.0.1".to_string());
    s
});

/// Heuristic: MCP servers often use paths like /mcp or run on known ports.
pub fn is_mcp_request(host: &str, path: &str) -> bool {
    path.to_lowercase().contains("mcp") || host.to_lowercase().contains("mcp")
}

/// Allowlisted MCP server origins.
pub fn allowed_origins() -> HashSet<String> {
    ALLOWED_ORIGINS.clone()
}

/// Check if the given host is in the allowlist.
pub fn origin_allowed(host: &str) -> bool {
    let host_lower = host.to_lowercase();
    if ALLOWED_ORIGINS.contains(&host_lower) {
        return true;
    }
    let host_no_port = host_lower.split(':').next().unwrap_or(&host_lower);
    ALLOWED_ORIGINS.contains(host_no_port)
}

/// Returns true if token passthrough is disabled (secure default).
pub fn token_passthrough_disabled() -> bool {
    true
}

/// Block private/internal IP ranges (SSRF mitigation).
pub fn would_be_ssrf(authority: &str) -> bool {
    let host = authority.split(':').next().unwrap_or(authority);
    if let Ok(ip) = IpAddr::from_str(host) {
        return is_private_or_internal(ip);
    }
    if host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" {
        return false;
    }
    false
}

fn is_private_or_internal(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            a.is_private()
                || a.is_loopback()
                || a.is_link_local()
                || a.is_broadcast()
                || a.octets()[0] == 169
        }
        IpAddr::V6(a) => a.is_loopback() || a.is_multicast(),
    }
}
