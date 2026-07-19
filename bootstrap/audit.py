#!/usr/bin/env python3
"""Independent standard-library audit for the clean-room specification.

Thin entry shim (BootstrapToolId path). The audit implementation lives in the
sibling ``audit`` package; this module loads it by absolute path with its own
submodule search location (no sys.path mutation) and re-exports its full public
surface, preserving the CLI argv contract exactly.
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_PACKAGE = Path(__file__).resolve().parent / "audit"
_SPEC = importlib.util.spec_from_file_location(
    "_audit_impl",
    _PACKAGE / "__init__.py",
    submodule_search_locations=[str(_PACKAGE)],
)
_IMPL = importlib.util.module_from_spec(_SPEC)
assert _SPEC.loader is not None
# Register during exec so the package's relative imports resolve through
# sys.modules; mirrors the loader in bootstrap/selftest.py load().
sys.modules["_audit_impl"] = _IMPL
_SPEC.loader.exec_module(_IMPL)

# Re-export the package's entire public surface into module globals so every
# harness `audit.<name>` attribute access resolves against this shim.
globals().update({k: v for k, v in vars(_IMPL).items() if not k.startswith("_")})

main = _IMPL.main


if __name__ == "__main__":
    raise SystemExit(main())
