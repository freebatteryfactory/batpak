#!/usr/bin/env python3
"""Independent portability self-test for the bootstrap freeze/audit tools.

Thin entry shim (BootstrapToolId path). The self-test implementation lives in
the sibling ``selftest`` package; this module loads it by absolute path with its
own submodule search location (no sys.path mutation) and re-exports its full
public surface, preserving the CLI argv contract exactly:

    selftest.py                                   plain run
    selftest.py --require-receipts <triple> [--emit <dir>]
    selftest.py --emit-evidence <dir>
    selftest.py --emit-run-metadata <path> <pairs...>
    selftest.py --verify-bundle <bundle> <root>
    selftest.py --confirm-promotion <6 args>
    selftest.py --supernova <campaign-root>
    selftest.py --supernova-compare <own-bundle> <candidate-bundle>
    selftest.py --e7-underwrite <out-dir> --campaign-root <dir> --tier0-bundle <dir>
    selftest.py --e7-compare <own-artifact> <candidate-artifact>
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_PACKAGE = Path(__file__).resolve().parent / "selftest"
_SPEC = importlib.util.spec_from_file_location(
    "_selftest_impl",
    _PACKAGE / "__init__.py",
    submodule_search_locations=[str(_PACKAGE)],
)
_IMPL = importlib.util.module_from_spec(_SPEC)
assert _SPEC.loader is not None
# Register during exec so the package's relative imports resolve through
# sys.modules; mirrors the loader in the selftest package's load().
sys.modules["_selftest_impl"] = _IMPL
_SPEC.loader.exec_module(_IMPL)

# Re-export the package's entire public surface into module globals so every
# harness `selftest.<name>` attribute access resolves against this shim.
globals().update({k: v for k, v in vars(_IMPL).items() if not k.startswith("_")})

main = _IMPL.main


if __name__ == "__main__":
    raise SystemExit(main())
