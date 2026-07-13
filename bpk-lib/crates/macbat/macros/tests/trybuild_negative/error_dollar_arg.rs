use macbat::Error;
#[derive(Error)]
enum E {
    #[error("value {a:1$}")]
    V { a: u32 },
}
fn main() {}
