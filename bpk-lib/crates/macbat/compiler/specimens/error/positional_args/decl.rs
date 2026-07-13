// Positional `{0}` / `{1}` rewrite over tuple variants.
enum Positional {
    #[error("plain: {0}")]
    Plain(String),
    #[error("two: {0} / {1}")]
    Two(u32, u32),
}
