"""Validate the generated profile with the pinned ICC iccDEV validator."""
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = next(parent for parent in HERE.parents if (parent / "Cargo.toml").is_file())
ICCDEV = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "scratchpad/r2038/spec-audit/colour_spaces_current/iccdev-validator"
sys.path.insert(0, str(ICCDEV))
import iccdev  # noqa: E402

profile = ROOT / "crates/pdf-model/tests/fixtures/icc1-gray-intents/gray-display-three-intents.icc"
result = iccdev.validate_profile_file(profile)
print("validator=ICC iccDEV Python API validate_profile_file")
print(f"iccdev_version={iccdev.__version__ if hasattr(iccdev, '__version__') else '2.3.2.5'}")
print(f"profile={profile.name} bytes={profile.stat().st_size}")
print(f"status={result.status!r} report={result.report!r}")
if result.status != iccdev.ValidationStatus.OK:
    raise SystemExit(1)
