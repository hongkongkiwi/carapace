#![no_main]

use libfuzzer_sys::fuzz_target;

use carapace::logging::redact::{redact_string, Redactor};

fuzz_target!(|data: &str| {
    // Fuzz the regex-based redaction engine with arbitrary strings.
    // The primary concern is ReDoS -- catastrophic backtracking in regex
    // patterns that causes the engine to hang on crafted inputs.
    //
    // libFuzzer will detect timeouts (default: 1200ms per input), which
    // catches ReDoS vulnerabilities.
    let _ = redact_string(data);

    // Also exercise the Redactor struct wrapper to ensure identical behavior.
    let redactor = Redactor::new();
    let _ = redactor.redact_string(data);
});
