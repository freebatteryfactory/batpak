// No `version` key — PAYLOAD_VERSION defaults to 1.
#[batpak(category = 0x1, type_id = 0x2)]
struct DefaultVersion {
    amount: i64,
}
