//! The satellite against the Nestor server's black-box conformance suite.
//!
//! The published server's `nestor satellites conformance`, run through uvx at the version in
//! `.nestor-version`, launches `my-satellite run --null-audio` once per scenario: pairing with a
//! device code and an enrollment code, refused codes, rate limits, identify, acks, text, audio both
//! ways and reconnection. Raise `.nestor-version` to check against a newer server.
//!
//! Skipped when `uvx` is not on the PATH, unless `NESTOR_CONFORMANCE=required` (CI).
#![allow(clippy::unwrap_used)] // test helpers fail loudly

use std::process::Command;

const SERVER_VERSION: &str = include_str!("../.nestor-version");
const SATELLITE: &str = env!("CARGO_BIN_EXE_my-satellite");

fn has_uvx() -> bool {
    Command::new("uvx").arg("--version").output().is_ok_and(|output| output.status.success())
}

#[test]
fn the_satellite_passes_the_servers_conformance_suite() {
    if !has_uvx() {
        let required = std::env::var("NESTOR_CONFORMANCE").is_ok_and(|v| v == "required");
        assert!(!required, "NESTOR_CONFORMANCE=required, but uvx is not on the PATH");
        eprintln!("skipped: no uvx to run the server's conformance suite");
        return;
    }
    let server = format!("nestor-home[voice]=={}", SERVER_VERSION.trim());
    let output = Command::new("uvx")
        .args(["--from", &server, "nestor", "satellites", "conformance", "--deadline", "20", "--"])
        .args([SATELLITE, "run", "--core", "{url}", "--room", "salon", "--token-file", "{state}/token.json"])
        .arg("--null-audio")
        .env("LC_ALL", "en_US.UTF-8")
        .output()
        .unwrap();
    let report = String::from_utf8_lossy(&output.stdout);
    println!("{report}");
    assert!(output.status.success(), "{report}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(!report.lines().any(|line| line.starts_with("FAIL")), "{report}");
    for check in [
        "PASS  receives the core's audio (A1, A2)",
        "PASS  sends its microphone audio (A1, A2)",
        "PASS  acknowledges intent.text within 1 s",
        "PASS  reconnects with the same token after the core closes its link (L5)",
        "PASS  pairs with the enrollment code of NESTOR_PAIRING_URI, no code typed (P3, P4)",
        "PASS  presents a refused enrollment code (used) only once (P5)",
        "PASS  keeps polling after a pending identify (I1)",
        "PASS  acknowledges intent.identify within 1 s",
    ] {
        assert!(report.lines().any(|line| line == check), "{check}: not run\n{report}");
    }
}
