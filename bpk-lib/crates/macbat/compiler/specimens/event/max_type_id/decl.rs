// Boundary: type_id = 0x0FFF (the 12-bit maximum accepted by validate_type_id).
#[batpak(category = 0x1, type_id = 0x0FFF)]
struct MaxTypeId {
    data: u32,
}
