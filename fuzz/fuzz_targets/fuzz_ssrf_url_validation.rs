#![no_main]

use libfuzzer_sys::fuzz_target;

use carapace::plugins::capabilities::{SsrfConfig, SsrfProtection};
use std::net::IpAddr;

fuzz_target!(|data: &str| {
    // Fuzz the URL validator with arbitrary strings.
    // This must never panic regardless of input.
    let _ = SsrfProtection::validate_url(data);

    // Also fuzz with Tailscale mode enabled to cover that code path.
    let config = SsrfConfig {
        allow_tailscale: true,
    };
    let _ = SsrfProtection::validate_url_with_config(data, &config);

    // If the input parses as an IP address, also fuzz the resolved-IP validator.
    if let Ok(ip) = data.parse::<IpAddr>() {
        let _ = SsrfProtection::validate_resolved_ip(&ip, "fuzz-host");
        let _ = SsrfProtection::validate_resolved_ip_with_config(&ip, "fuzz-host", &config);
    }
});
