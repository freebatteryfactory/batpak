// Lifted (subset) from bvisor RequirementKind — a real AllVariants adopter.
enum RequirementKind {
    Filesystem,
    NetworkDenyAll,
    NetworkAllowList,
    Environment,
    LaunchWorkload,
}
