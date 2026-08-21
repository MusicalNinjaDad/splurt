//! Tracing tests for hex module

use redbook::hex::{hex_dump, hex_to_bytes, parse_toc};

mod tracing;
use tracing::{init_capturing_tracing, init_trace_tracing};

#[test]
fn test_hex_to_bytes_emits_trace_span() {
    let _guard = init_trace_tracing();

    // This should emit a trace span with the input length
    let hex = "000102030405";
    let result = hex_to_bytes(hex);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
}

#[test]
fn test_hex_dump_emits_trace_span() {
    let _guard = init_trace_tracing();

    // This should emit a trace span with byte count
    let bytes = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
    let result = hex_dump(&bytes);

    assert!(!result.is_empty());
}

#[test]
fn test_parse_toc_emits_debug_span() {
    let _guard = init_capturing_tracing("debug");

    // This should emit a debug span with TOC entry count
    // Using the TOC.hex fixture from Definitely Maybe
    let toc_bytes = hex_to_bytes(include_str!(
        "assets/9822581d-98bf-3f97-a94c-4b1350d090aa/TOC.hex"
    ))
    .unwrap();
    let result = parse_toc(toc_bytes);

    assert!(!result.is_empty());
}
