# RosettaStune

RosettaStune is a lightweight unit registry for cross-schema linked data. It uses a Rust core for fast alias resolution and a Python layer for optional conversions to Pint, Astropy, and SymPy.

## Docs

- Unit alias policy and contribution guide: [docs/unit-alias-policy.md](docs/unit-alias-policy.md)
- Agent and contributor workflow: [AGENTS.md](AGENTS.md)

## Design

- `python/rosettastune/data/lexicon.jsonld` is the language-agnostic source of truth.
- The Rust extension parses the JSON-LD lexicon and resolves identifiers locally.
- The Python wrapper converts resolved canonical units into whichever unit library is available.

## Backends

- Pint for runtime array math and engineering calculations.
- Astropy for strict scientific unit handling.
- SymPy for symbolic expressions and algebra.

## Quick Start

### Dev Container

- Open this repository in the included dev container: [.devcontainer/devcontainer.json](.devcontainer/devcontainer.json)
- The container installs Python, Rust, `llvm-tools-preview`, and `cargo-llvm-cov` during `postCreateCommand`.

### Local Setup

```bash
make install-dev
```

If your Python executable is not the project venv default, override it:

```bash
make install-dev PYTHON=/path/to/python
```

## Development Workflow

The project includes a task-oriented Makefile: [Makefile](Makefile)

```bash
make help
make fmt
make test
make ci
```

Common tasks:

- `make snapshot-update` refreshes Rust insta snapshots.
- `make coverage-python` generates `coverage.xml`, `coverage.txt`, and `pytest-junit.xml` via pytest defaults.
- `make coverage-rust` generates `rust-coverage.lcov` (requires `cargo-llvm-cov`).

## Manual Commands

- Build the extension with Maturin.
- Run Rust tests and snapshot checks with Cargo.
- Run Python tests with Pytest.

```bash
cargo fmt --check
cargo test
python -m pip install -e '.[dev]'
python -m pytest
```

## CI And Coverage

- CI workflow: [.github/workflows/ci.yml](.github/workflows/ci.yml)
- Coverage thresholds source of truth: [Cargo.toml](Cargo.toml) in `[package.metadata.coverage]`
- CI enforces both:
	- Python coverage minimum (`python_min`)
	- Rust line coverage minimum (`rust_min`) via `cargo llvm-cov`
- On published releases, CI uploads coverage artifacts to the release:
	- `pytest-junit.xml`
	- `coverage.xml`
	- `coverage.txt`
	- `rust-coverage.lcov`
	- `rust-coverage.txt`

## Troubleshooting

- `ModuleNotFoundError: No module named rosettastune` while running tests:
	- Use the project venv interpreter and install editable deps first:

```bash
make install-dev
make test
```

	- Or override `PYTHON` explicitly:

```bash
make test PYTHON=/path/to/python
```

- `error: no such command: llvm-cov`:
	- Ensure `cargo-llvm-cov` is installed and `llvm-tools-preview` is available.
	- The dev container and CI install both automatically.

- PyO3 import/init mismatch errors:
	- Ensure Maturin and PyO3 module names match:
		- `pyproject.toml`: `module-name = "rosettastune._rosettastune"`
		- `src/lib.rs`: `#[pymodule] fn _rosettastune(...)`

- Snapshot drift in Rust tests:

```bash
INSTA_UPDATE=always cargo test
```

## Extending The Lexicon

1. Add canonical entry and aliases in `python/rosettastune/data/lexicon.jsonld`.
2. Add backend mappings in `python/rosettastune/api.py` for Pint, Astropy, and SymPy.
3. Update or regenerate snapshots with `INSTA_UPDATE=always cargo test`.
4. Add or adjust Python tests in `tests/test_rosettastune.py`.
