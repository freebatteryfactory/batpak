# Support

## Version support & on-disk / wire compatibility (pre-1.0)

batpak is pre-1.0. Within the `0.x` line the on-disk store format and the
netbat wire format may change **without a migration path**. In particular,
**0.10.0 does not guarantee reading stores or frames written by earlier batpak
versions; on-disk and wire formats may change without migration until 1.0.**
Treat a store directory and a peer as tied to the exact batpak version that
wrote them until the format stabilizes at 1.0. Legacy-format readers are
removed as debt whenever they cease to carry a live capability (see the
CHANGELOG for the specific readers dropped in each release).

- Usage questions: open a discussion with a small runnable example:
  <https://github.com/freebatteryfactory/batpak/discussions>.
- Bug reports: use the issue chooser and include `just doctor` output when relevant:
  <https://github.com/freebatteryfactory/batpak/issues/new/choose>.
- Performance questions: include the benchmark surface, whether you saved or
  compared against a baseline, and the relevant benchmark or conformance context
  from `12_CONFORMANCE.md`.
