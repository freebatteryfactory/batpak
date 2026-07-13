// Lifted from core/src/coordinate/mod.rs (CoordinateError). Enum-only Error surface:
// unit variants + a named-field variant exercising `{len}`/`{max}` interpolation.
enum CoordinateError {
    #[error("entity cannot be empty")]
    EmptyEntity,
    #[error("entity length {len} exceeds maximum {max}")]
    EntityTooLong { len: usize, max: usize },
    #[error("coordinate component contains a NUL byte")]
    NulByte,
}
