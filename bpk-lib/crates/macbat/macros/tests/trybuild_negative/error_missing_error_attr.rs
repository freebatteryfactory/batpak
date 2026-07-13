use macbat::Error;
#[derive(Error)]
enum E {
    #[error("ok")]
    A,
    B,
}
fn main() {}
