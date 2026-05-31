# AGENTS.md

This repository supports coding agents and contributors working on RosettaStune.

## Project Summary

RosettaStune is a Rust/Python hybrid package:

- Rust core resolver in `src/lib.rs`.
- Python package in `python/rosettastune/`.
- Build backend: Maturin + PyO3.
- Linked data lexicon source of truth: `python/rosettastune/data/lexicon.jsonld`.

## Ground Rules

- Keep changes minimal and focused.
- Preserve the existing public API unless explicitly asked to change it.
- Prefer additive changes to the unit lexicon.
- Avoid destructive git operations.

## Build And Test Commands

Run from repository root:

```bash
make help
make fmt
make test
make ci
```

Python environment and tests:

```bash
make install-dev
make test-python
```

If lexicon snapshots need refresh:

```bash
make snapshot-update
```

Rust/Python toolchain defaults:

- Rust toolchain and components are pinned in `rust-toolchain.toml`.
- Dev container setup is defined in `.devcontainer/devcontainer.json`.

## Where To Edit

- Add or update unit aliases in `python/rosettastune/data/lexicon.jsonld`.
- Update backend unit mappings in `python/rosettastune/api.py`.
- Keep Rust resolver behavior aligned in `src/lib.rs`.
- Add tests in:
  - `tests/lexicon_snapshot.rs` for Rust snapshot coverage.
  - `tests/test_rosettastune.py` for Python behavior (fixtures, parametrize, mocks).

## Common Pitfalls

- PyO3 module name must match Maturin config:
  - `pyproject.toml`: `module-name = "rosettastune._rosettastune"`
  - `src/lib.rs`: `#[pymodule] fn _rosettastune(...)`
- Keep `pyo3/extension-module` in Maturin features, not Cargo dependency features, so `cargo test` links correctly.
- SymPy cannot represent Celsius as a simple linear token; this should continue to raise `ValueError`.
- `make test` uses the project venv Python by default; override with `PYTHON=/path/to/python` if needed.
- `cargo llvm-cov` requires both `cargo-llvm-cov` and `llvm-tools-preview`.

## Documentation

- Main guide: `README.md`
- Unit policy: `docs/unit-alias-policy.md`
- Task automation: `Makefile`

## CI Expectations

Pull requests should keep all checks passing:

- `cargo fmt --check`
- `cargo test`
- `pytest tests/ --cov-fail-under=<python_min>`
- `cargo llvm-cov --fail-under-lines <rust_min>`

Coverage thresholds are sourced from `Cargo.toml` under `[package.metadata.coverage]`.

## Release Checklist

Before publishing a release:

1. Run local checks:

```bash
make ci
```

2. Confirm version metadata updates are complete (`Cargo.toml`, `pyproject.toml`).
3. Confirm `[package.metadata.coverage]` thresholds in `Cargo.toml` are intended.
4. Create and publish the GitHub release.

After publishing:

1. Verify the `release-assets` job in `.github/workflows/ci.yml` succeeds.
2. Verify uploaded release artifacts include:
  - `pytest-junit.xml`
  - `coverage.xml`
  - `coverage.txt`
  - `rust-coverage.lcov`
  - `rust-coverage.txt`
3. If artifacts are missing, inspect CI logs for coverage gate failures or environment setup issues.
