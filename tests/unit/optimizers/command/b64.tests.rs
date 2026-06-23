//! Unit tests for the minimal base64 codec, in AAA form. Compiled into `snip_lib`
//! via a `#[path]` include in `src/commands/route/b64.rs`.

use assert2::check;

use super::{decode, encode};

#[test]
fn matches_a_known_vector() {
    // Arrange + Act + Assert (RFC 4648 example)
    check!(encode(b"foobar") == "Zm9vYmFy");
    check!(decode("Zm9vYmFy") == Some(b"foobar".to_vec()));
}

#[test]
fn round_trips_arbitrary_bytes_including_shell_metacharacters() {
    // Arrange: the exact bytes a command might contain
    let cases: &[&[u8]] = &[
        b"",
        b"a",
        b"ab",
        b"abc",
        b"git status && echo 'a | b' > $(date)",
        &[0, 255, 10, 13, 127],
    ];

    // Act + Assert
    for &bytes in cases {
        let round = decode(&encode(bytes));
        assert!(
            round.as_deref() == Some(bytes),
            "round-trip failed for {bytes:?}"
        );
    }
}

#[test]
fn rejects_invalid_characters() {
    // Arrange + Act + Assert: a space is not in the alphabet
    check!(decode("not valid!").is_none());
}

#[test]
fn rejects_a_dangling_single_symbol_chunk() {
    // Arrange + Act + Assert: one stray symbol can't encode a byte → None
    check!(decode("A").is_none());
}

#[test]
fn round_trips_bytes_whose_encoding_uses_plus_and_slash() {
    // Arrange: 0xFB encodes to "+w==" (exercises the `+` symbol); three 0xFF
    // bytes encode to "////" (the `/` symbol)
    // Act
    let plus = encode(&[0xFB]);
    let slash = encode(&[0xFF, 0xFF, 0xFF]);

    // Assert: both special symbols appear and decode back losslessly
    check!(plus == "+w==");
    check!(slash == "////");
    check!(decode("+w==") == Some(vec![0xFB]));
    check!(decode("////") == Some(vec![0xFF, 0xFF, 0xFF]));
}
