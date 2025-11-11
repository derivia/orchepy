use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::net::IpAddr;
use tracing::{debug, warn};

#[derive(Clone)]
pub struct WhitelistConfig {
    pub enabled: bool,
    pub allowed_ips: Vec<IpAddr>,
}

impl WhitelistConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("WHITELIST_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let allowed_ips = std::env::var("WHITELIST_IPS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .collect();

        debug!("Whitelist enabled: {}", enabled);
        debug!("Allowed IPs: {:?}", allowed_ips);

        Self {
            enabled,
            allowed_ips,
        }
    }

    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        if !self.enabled {
            return true;
        }

        if ip.is_loopback() {
            return true;
        }

        self.allowed_ips.contains(ip)
    }
}

pub async fn whitelist_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let config = WhitelistConfig::from_env();

    if !config.enabled {
        return Ok(next.run(request).await);
    }

    let ip = extract_client_ip(&request);

    match ip {
        Some(client_ip) => {
            if config.is_allowed(&client_ip) {
                debug!("Request from allowed IP: {}", client_ip);
                Ok(next.run(request).await)
            } else {
                warn!("Blocked request from unauthorized IP: {}", client_ip);
                Err((
                    StatusCode::FORBIDDEN,
                    format!("Access denied from IP: {}", client_ip),
                ))
            }
        }
        None => {
            warn!("Could not extract client IP from request");
            Err((
                StatusCode::FORBIDDEN,
                "Could not determine client IP".to_string(),
            ))
        }
    }
}

fn extract_client_ip(request: &Request) -> Option<IpAddr> {
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }

    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    request
        .extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip())
}
