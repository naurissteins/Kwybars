mod legacy;
mod parse;

use super::{Parsed, parse as parse_config};

/// parses text that must be valid
fn parse_ok(raw: &str) -> Parsed {
    match parse_config(raw) {
        Ok(parsed) => parsed,
        Err(err) => panic!("config should parse, got: {err}"),
    }
}

/// parses text that must fail and returns the rendered error
fn parse_err(raw: &str) -> String {
    match parse_config(raw) {
        Ok(parsed) => panic!("config should fail, got: {parsed:?}"),
        Err(err) => err.to_string(),
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-5,
        "expected {expected}, got {actual}"
    );
}
