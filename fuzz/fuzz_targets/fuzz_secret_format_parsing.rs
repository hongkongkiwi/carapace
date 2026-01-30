#![no_main]

use libfuzzer_sys::fuzz_target;

use carapace::config::secrets::{is_encrypted, parse_encrypted, SecretStore};

fuzz_target!(|data: &str| {
    // Fuzz the enc:v1: format parser with arbitrary strings.
    // This must never panic regardless of input -- only return Ok/Err.
    let _ = parse_encrypted(data);

    // Also fuzz the is_encrypted check.
    let _ = is_encrypted(data);

    // Fuzz the full decrypt path (which calls parse_encrypted internally).
    // Use a fixed password/salt to avoid the expensive PBKDF2 key derivation
    // dominating fuzz throughput.
    let salt = [0xABu8; 16];
    let store = SecretStore::from_password_and_salt(b"fuzz-password", &salt);
    let _ = store.decrypt(data);
});
