// NOTE: this negative is ACCEPTED by the grammar and fails only at emitted-token
// typeck; in the core-free macro lane its .stderr golden also carries unresolved
// `::batpak` paths. Meaningful as a pure handler-signature negative only post-J1.
use macbat::Projection;
#[derive(Projection)]
#[batpak(input = JsonValueInput)]
#[batpak(event = Incremented, handler = wrong)]
struct P {
    v: i64,
}
impl P {
    fn wrong(&self) {}
}
fn main() {}
