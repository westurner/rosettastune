from __future__ import annotations

from dataclasses import dataclass
from importlib import resources
from typing import Any, Optional

try:
    from ._rosettastune import LexiconRegistry, ResolvedUnit
except ImportError as exc:  # pragma: no cover - import path depends on build step
    raise ImportError(
        "RosettaStune native extension is not built. Install with Maturin or build the wheel first."
    ) from exc

try:
    import pint

    _HAS_PINT = True
except ImportError:  # pragma: no cover - optional dependency
    _HAS_PINT = False

try:
    import astropy.units as u

    _HAS_ASTROPY = True
except ImportError:  # pragma: no cover - optional dependency
    _HAS_ASTROPY = False

try:
    import sympy.physics.units as su

    _HAS_SYMPY = True
except ImportError:  # pragma: no cover - optional dependency
    _HAS_SYMPY = False


def _load_lexicon_text(path: Optional[str] = None) -> str:
    if path is not None:
        return resources.files(__package__).joinpath(path).read_text(encoding="utf-8")

    return resources.files(__package__).joinpath("data/lexicon.jsonld").read_text(encoding="utf-8")


def _pint_unit(canonical: str, registry: "pint.UnitRegistry"):
    mapping = {
        "kg": registry.kilogram,
        "g": registry.gram,
        "mm": registry.millimeter,
        "m": registry.meter,
        "L": registry.liter,
        "s": registry.second,
        "min": registry.minute,
        "h": registry.hour,
        "W": registry.watt,
        "kW": registry.kilowatt,
        "kWh": registry.kilowatt_hour,
        "Pa": registry.pascal,
        "J/kg": registry.joule / registry.kilogram,
        "degC": registry.degC,
        "K": registry.kelvin,
    }
    if canonical not in mapping:
        raise ValueError(f"no Pint mapping for canonical unit {canonical!r}")
    return mapping[canonical]


def _astropy_unit(canonical: str):
    mapping = {
        "kg": u.kg,
        "g": u.g,
        "mm": u.mm,
        "m": u.m,
        "L": u.L,
        "s": u.s,
        "min": u.min,
        "h": u.h,
        "W": u.W,
        "kW": u.kW,
        "kWh": u.kW * u.h,
        "Pa": u.Pa,
        "J/kg": u.J / u.kg,
        "degC": u.deg_C,
        "K": u.K,
    }
    if canonical not in mapping:
        raise ValueError(f"no Astropy mapping for canonical unit {canonical!r}")
    return mapping[canonical]


def _sympy_unit(canonical: str):
    mapping = {
        "kg": su.kilogram,
        "g": su.gram,
        "mm": su.millimeter,
        "m": su.meter,
        "L": su.liter,
        "s": su.second,
        "min": su.minute,
        "h": su.hour,
        "W": su.watt,
        "kW": su.kilo * su.watt,
        "kWh": su.kilo * su.watt * su.hour,
        "Pa": su.pascal,
        "J/kg": su.joule / su.kilogram,
        "K": su.kelvin,
    }
    if canonical == "degC":
        raise ValueError("SymPy does not model Celsius as a simple linear unit token")
    if canonical not in mapping:
        raise ValueError(f"no SymPy mapping for canonical unit {canonical!r}")
    return mapping[canonical]


@dataclass(frozen=True)
class BackendUnit:
    backend: str
    unit: Any


class RosettaStuneRegistry:
    def __init__(self, lexicon_path: Optional[str] = None, lexicon_text: Optional[str] = None):
        if lexicon_text is None:
            lexicon_text = _load_lexicon_text(lexicon_path)

        self._core = LexiconRegistry(lexicon_text)
        self._pint_registry = pint.UnitRegistry() if _HAS_PINT else None

    def resolve(self, identifier: str) -> ResolvedUnit:
        return self._core.resolve(identifier)

    def known_identifiers(self) -> list[str]:
        return self._core.known_identifiers()

    def to_pint(self, identifier: str):
        if not _HAS_PINT or self._pint_registry is None:
            raise ImportError("Pint is not installed in this environment.")
        return _pint_unit(self.resolve(identifier).canonical, self._pint_registry)

    def to_astropy(self, identifier: str):
        if not _HAS_ASTROPY:
            raise ImportError("Astropy is not installed in this environment.")
        return _astropy_unit(self.resolve(identifier).canonical)

    def to_sympy(self, identifier: str):
        if not _HAS_SYMPY:
            raise ImportError("SymPy is not installed in this environment.")
        return _sympy_unit(self.resolve(identifier).canonical)

    def wrap_array_pint(self, array, identifier: str):
        return array * self.to_pint(identifier)

    def wrap_array_astropy(self, array, identifier: str):
        return array * self.to_astropy(identifier)

    def wrap_symbol_sympy(self, symbol_name: str, identifier: str):
        if not _HAS_SYMPY:
            raise ImportError("SymPy is not installed in this environment.")
        import sympy as sp

        return sp.Symbol(symbol_name) * self.to_sympy(identifier)


def load_default_registry() -> RosettaStuneRegistry:
    return RosettaStuneRegistry()
