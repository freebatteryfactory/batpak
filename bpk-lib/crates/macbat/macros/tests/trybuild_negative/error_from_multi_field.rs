use macbat::Error;
#[derive(Debug)]
struct Leaf;
#[derive(Error)]
enum E {
    #[error("multi")]
    V(#[from] Leaf, u32),
}
fn main() {}
