use macbat::Error;
#[derive(Debug)]
struct Leaf;
#[derive(Error)]
enum E {
    #[error("two sources")]
    V {
        #[source]
        a: Leaf,
        #[source]
        b: Leaf,
    },
}
fn main() {}
