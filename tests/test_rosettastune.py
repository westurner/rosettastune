from __future__ import annotations

from unittest.mock import patch

import pytest

from rosettastune import RosettaStuneRegistry
import rosettastune.api as api


@pytest.fixture()
def registry() -> RosettaStuneRegistry:
    return RosettaStuneRegistry()


@pytest.mark.parametrize(
    "identifier, canonical",
    [
        ("qudt:KILO_GM", "kg"),
        ("qudt:GM", "g"),
        ("om:kilogram", "kg"),
        ("unece:KGM", "kg"),
        ("om:litre", "L"),
        ("unece:PAL", "Pa"),
        ("iof_adapter:time/h", "h"),
        ("qudt:KiloW_HR", "kWh"),
        ("om:joulePerKilogram", "J/kg"),
        ("qudt:DEG_C", "degC"),
        ("unece:KEL", "K"),
    ],
)
def test_resolve_aliases(registry: RosettaStuneRegistry, identifier: str, canonical: str) -> None:
    resolved = registry.resolve(identifier)
    assert resolved.canonical == canonical
    assert identifier in resolved.aliases


@pytest.mark.parametrize(
    "identifier, expected_backend",
    [
        ("qudt:KILO_GM", "pint"),
        ("unece:WTT", "astropy"),
        ("om:joulePerKilogram", "sympy"),
    ],
)
def test_optional_backends_are_supported(registry: RosettaStuneRegistry, identifier: str, expected_backend: str) -> None:
    if expected_backend == "pint":
        try:
            unit = registry.to_pint(identifier)
        except ImportError:
            pytest.skip("Pint is not installed")
        assert str(unit) in {"kilogram", "kg"}
    elif expected_backend == "astropy":
        try:
            unit = registry.to_astropy(identifier)
        except ImportError:
            pytest.skip("Astropy is not installed")
        assert str(unit) in {"W", "watt"}
    else:
        try:
            unit = registry.to_sympy(identifier)
        except ImportError:
            pytest.skip("SymPy is not installed")
        assert "joule" in str(unit)


def test_temperature_resolves_even_if_sympy_cannot_render_it(registry: RosettaStuneRegistry) -> None:
    resolved = registry.resolve("qudt:DEG_C")
    assert resolved.canonical == "degC"

    with pytest.raises(ValueError):
        registry.to_sympy("qudt:DEG_C")


@pytest.mark.parametrize(
    "backend_name, method_name, identifier",
    [
        ("_HAS_PINT", "to_pint", "qudt:KILO_GM"),
        ("_HAS_ASTROPY", "to_astropy", "qudt:KILO_GM"),
        ("_HAS_SYMPY", "to_sympy", "qudt:KILO_GM"),
    ],
)
def test_missing_backend_raises_import_error(
    registry: RosettaStuneRegistry,
    monkeypatch: pytest.MonkeyPatch,
    backend_name: str,
    method_name: str,
    identifier: str,
) -> None:
    monkeypatch.setattr(api, backend_name, False)

    with pytest.raises(ImportError):
        getattr(registry, method_name)(identifier)


def test_wrap_array_pint_uses_conversion_method(registry: RosettaStuneRegistry) -> None:
    with patch.object(registry, "to_pint", return_value=2) as mocked:
        result = registry.wrap_array_pint(3, "qudt:KILO_GM")

    assert result == 6
    mocked.assert_called_once_with("qudt:KILO_GM")


def test_wrap_array_astropy_uses_conversion_method(registry: RosettaStuneRegistry) -> None:
    with patch.object(registry, "to_astropy", return_value=4) as mocked:
        result = registry.wrap_array_astropy(3, "qudt:KILO_GM")

    assert result == 12
    mocked.assert_called_once_with("qudt:KILO_GM")


def test_wrap_symbol_sympy_calls_backend(registry: RosettaStuneRegistry) -> None:
    if not api._HAS_SYMPY:
        pytest.skip("SymPy is not installed")

    with patch.object(registry, "to_sympy", return_value=5) as mocked_to_sympy:
        with patch("sympy.Symbol", return_value=7) as mocked_symbol:
            result = registry.wrap_symbol_sympy("m_flow", "qudt:KILO_GM")

    assert result == 35
    mocked_to_sympy.assert_called_once_with("qudt:KILO_GM")
    mocked_symbol.assert_called_once_with("m_flow")


def test_wrap_symbol_sympy_missing_backend_raises_import_error(
    registry: RosettaStuneRegistry,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(api, "_HAS_SYMPY", False)

    with pytest.raises(ImportError):
        registry.wrap_symbol_sympy("m_flow", "qudt:KILO_GM")


def test_known_identifiers_includes_expected_alias(registry: RosettaStuneRegistry) -> None:
    identifiers = registry.known_identifiers()
    assert "qudt:KILO_GM" in identifiers


def test_load_lexicon_text_with_relative_path() -> None:
    text = api._load_lexicon_text("data/lexicon.jsonld")
    assert "@graph" in text


@pytest.mark.parametrize("backend", ["pint", "astropy", "sympy"])
def test_unknown_canonical_raises_value_error(registry: RosettaStuneRegistry, backend: str) -> None:
    if backend == "pint":
        if not api._HAS_PINT or registry._pint_registry is None:
            pytest.skip("Pint is not installed")
        with pytest.raises(ValueError):
            api._pint_unit("not_a_unit", registry._pint_registry)
    elif backend == "astropy":
        if not api._HAS_ASTROPY:
            pytest.skip("Astropy is not installed")
        with pytest.raises(ValueError):
            api._astropy_unit("not_a_unit")
    else:
        if not api._HAS_SYMPY:
            pytest.skip("SymPy is not installed")
        with pytest.raises(ValueError):
            api._sympy_unit("not_a_unit")


def test_load_default_registry_returns_registry_instance() -> None:
    default_registry = api.load_default_registry()
    assert isinstance(default_registry, RosettaStuneRegistry)
