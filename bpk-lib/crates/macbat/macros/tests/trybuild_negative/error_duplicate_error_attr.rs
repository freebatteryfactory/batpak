use macbat::Error;
#[derive(Error)]
enum E {
    #[error("a")]
    #[error("b")]
    A,
}
fn main() {}
