// `#[source]` on a Box<dyn Error> — source() must expose the inner via `.as_ref()`.
enum Boxed {
    #[error("boxed: {source}")]
    Wrap {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("bare {code}")]
    Bare { code: u16 },
}
