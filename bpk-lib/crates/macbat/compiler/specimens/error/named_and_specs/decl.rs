// Format specs: `{payload:?}` Debug and `{byte:02x}` hex width.
enum Specs {
    #[error("debug {payload:?}")]
    Debug { payload: Vec<u8> },
    #[error("hex 0x{byte:02x}")]
    Hex { byte: u8 },
}
