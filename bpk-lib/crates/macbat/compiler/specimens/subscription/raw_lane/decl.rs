// Raw msgpack input lane.
#[batpak(input = RawMsgpackInput, error = StoreError)]
#[batpak(event = RawEvent, handler = on_raw)]
struct RawLane {
    count: u64,
}
