// `#[from]` generates `impl From<Leaf>` and wires the source.
enum FromError {
    #[error("io failed: {0}")]
    Io(#[from] Leaf),
    #[error("other {0}")]
    Other(u32),
}
