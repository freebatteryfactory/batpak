//! Behaviour tests for `#[derive(Error)]` — the IndependentOracle (O-16).
//!
//! Moved VERBATIM from `batpak-macros`' `tests/error_derive.rs`; only the import
//! path changed (`batpak_macros` -> `macbat`). The `Error` surface emits ONLY
//! `::core`/`::std` paths, so this compiles AND runs for real in the core-free
//! macro lane. It is the strongest single parity anchor in crate 1: the central
//! guarantee is byte-identical rendering + identical `source()` wiring versus a
//! hand-rolled impl, so several cases assert the derived type against a
//! hand-written reference with the same shape.

use std::error::Error;
use std::fmt;

use macbat::Error;

// ─── A leaf error to hang `source()` chains from ─────────────────────────────

#[derive(Debug)]
struct Leaf(&'static str);

impl fmt::Display for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "leaf: {}", self.0)
    }
}

impl Error for Leaf {}

// ─── Named / unit / spec / Debug-arg coverage ────────────────────────────────

#[derive(Debug, Error)]
enum Kinds {
    #[error("unit variant")]
    Unit,
    #[error("named {name} = {value}")]
    Named { name: &'static str, value: u32 },
    #[error("debug {payload:?}")]
    Debug { payload: Vec<u8> },
    #[error("hex 0x{byte:02x}")]
    Hex { byte: u8 },
    // `\`-continuation + leading whitespace: proves the literal survives verbatim.
    #[error(
        "first line \
             second clause {tail}"
    )]
    Continued { tail: u32 },
}

/// Hand-rolled reference with the SAME variants and SAME literals.
enum KindsRef {
    Unit,
    Named { name: &'static str, value: u32 },
    Debug { payload: Vec<u8> },
    Hex { byte: u8 },
    Continued { tail: u32 },
}

impl fmt::Display for KindsRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "unit variant"),
            Self::Named { name, value } => write!(f, "named {name} = {value}"),
            Self::Debug { payload } => write!(f, "debug {payload:?}"),
            Self::Hex { byte } => write!(f, "hex 0x{byte:02x}"),
            Self::Continued { tail } => write!(
                f,
                "first line \
                 second clause {tail}"
            ),
        }
    }
}

#[test]
fn renders_byte_identical_to_reference() {
    let cases: &[(Kinds, KindsRef)] = &[
        (Kinds::Unit, KindsRef::Unit),
        (
            Kinds::Named {
                name: "n",
                value: 7,
            },
            KindsRef::Named {
                name: "n",
                value: 7,
            },
        ),
        (
            Kinds::Debug {
                payload: vec![1, 2, 3],
            },
            KindsRef::Debug {
                payload: vec![1, 2, 3],
            },
        ),
        (Kinds::Hex { byte: 0x2f }, KindsRef::Hex { byte: 0x2f }),
        (
            Kinds::Continued { tail: 9 },
            KindsRef::Continued { tail: 9 },
        ),
    ];
    for (derived, reference) in cases {
        assert_eq!(derived.to_string(), reference.to_string());
    }
}

#[test]
fn spot_check_exact_strings() {
    assert_eq!(Kinds::Unit.to_string(), "unit variant");
    assert_eq!(
        Kinds::Named {
            name: "widget",
            value: 42
        }
        .to_string(),
        "named widget = 42"
    );
    assert_eq!(Kinds::Hex { byte: 0x2f }.to_string(), "hex 0x2f");
    assert_eq!(
        Kinds::Continued { tail: 3 }.to_string(),
        "first line second clause 3"
    );
}

#[test]
fn no_source_variants_report_none() {
    let err = Kinds::Unit;
    assert!(err.source().is_none());
    let err = Kinds::Named {
        name: "n",
        value: 0,
    };
    assert!(err.source().is_none());
}

// ─── Tuple variants + `{0}` ──────────────────────────────────────────────────

#[derive(Debug, Error)]
enum Tuples {
    #[error("plain: {0}")]
    Plain(String),
    #[error("two: {0} / {1}")]
    Two(u32, u32),
}

#[test]
fn tuple_positional_args() {
    assert_eq!(Tuples::Plain("hi".to_owned()).to_string(), "plain: hi");
    assert_eq!(Tuples::Two(3, 4).to_string(), "two: 3 / 4");
    assert!(Tuples::Plain(String::new()).source().is_none());
}

// ─── `#[source]` wiring: concrete, tuple, and Box<dyn Error> ─────────────────

#[derive(Debug, Error)]
enum Sourced {
    #[error("wrapping {context}: {source}")]
    Named {
        context: &'static str,
        #[source]
        source: Leaf,
    },
    #[error("tuple decode: {0}")]
    Tuple(#[source] Leaf),
    #[error("boxed: {source}")]
    Boxed {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
    #[error("no source here {code}")]
    Bare { code: u16 },
}

#[test]
fn source_wiring_matches_hand_rolled() {
    let named = Sourced::Named {
        context: "ctx",
        source: Leaf("inner"),
    };
    assert_eq!(named.to_string(), "wrapping ctx: leaf: inner");
    let src = named.source().expect("named has a source");
    assert_eq!(src.to_string(), "leaf: inner");
    assert!(src.is::<Leaf>());

    let tuple = Sourced::Tuple(Leaf("t"));
    assert_eq!(tuple.to_string(), "tuple decode: leaf: t");
    assert!(tuple.source().expect("tuple has a source").is::<Leaf>());

    let boxed = Sourced::Boxed {
        source: Box::new(Leaf("b")),
    };
    assert_eq!(boxed.to_string(), "boxed: leaf: b");
    let bsrc = boxed.source().expect("boxed has a source");
    // The inner dyn error is exposed directly (not the Box wrapper).
    assert!(bsrc.is::<Leaf>());
    assert_eq!(bsrc.to_string(), "leaf: b");

    assert!(Sourced::Bare { code: 1 }.source().is_none());
}

// ─── `#[from]` generates `impl From` and wires the source ────────────────────

#[derive(Debug, Error)]
enum FromEnum {
    #[error("io failed: {0}")]
    Io(#[from] Leaf),
    #[error("other {0}")]
    Other(u32),
}

#[test]
fn from_impl_generated() {
    let err: FromEnum = Leaf("boom").into();
    assert_eq!(err.to_string(), "io failed: leaf: boom");
    assert!(err
        .source()
        .expect("From variant carries a source")
        .is::<Leaf>());
    assert!(FromEnum::Other(1).source().is_none());
}

// ─── Compile-time proof the derived types are real `std::error::Error` ───────

fn assert_is_error<E: Error>() {}

#[test]
fn derived_types_are_errors() {
    assert_is_error::<Kinds>();
    assert_is_error::<Tuples>();
    assert_is_error::<Sourced>();
    assert_is_error::<FromEnum>();
}
