use macbat::Error;
#[derive(Error)]
enum E {
    #[error("bad {")]
    V,
}
fn main() {}
