use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const DEFAULT_LEXICON: &str = include_str!("../python/rosettastune/data/lexicon.jsonld");

#[derive(Debug, Clone, Deserialize)]
struct LexiconDocument {
    #[serde(rename = "@context")]
    context: HashMap<String, String>,
    #[serde(rename = "@graph")]
    graph: Vec<LexiconEntryInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct LexiconEntryInput {
    #[serde(rename = "@id")]
    id: String,
    canonical: String,
    dimension: Option<String>,
    aliases: Option<Vec<String>>,
}

#[pyclass]
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedUnit {
    #[pyo3(get)]
    pub identifier: String,
    #[pyo3(get)]
    pub iri: String,
    #[pyo3(get)]
    pub canonical: String,
    #[pyo3(get)]
    pub dimension: Option<String>,
    #[pyo3(get)]
    pub aliases: Vec<String>,
}

impl ResolvedUnit {
    fn new(
        identifier: String,
        iri: String,
        canonical: String,
        dimension: Option<String>,
        aliases: Vec<String>,
    ) -> Self {
        Self {
            identifier,
            iri,
            canonical,
            dimension,
            aliases,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct LexiconRegistry {
    lookup: HashMap<String, ResolvedUnit>,
    entries: Vec<ResolvedUnit>,
}

impl LexiconRegistry {
    pub fn from_jsonld_str(jsonld: &str) -> PyResult<Self> {
        let document: LexiconDocument = serde_json::from_str(jsonld)
            .map_err(|error| PyValueError::new_err(format!("invalid JSON-LD lexicon: {error}")))?;

        let mut lookup: HashMap<String, ResolvedUnit> = HashMap::new();
        let mut entries: Vec<ResolvedUnit> = Vec::new();

        for entry in document.graph {
            let expanded_identifier = expand_identifier(&entry.id, &document.context);
            let mut aliases = Vec::new();
            let mut seen = HashSet::new();

            register_identifier(&mut aliases, &mut seen, &entry.id);
            register_identifier(&mut aliases, &mut seen, &expanded_identifier);
            register_identifier(&mut aliases, &mut seen, &entry.canonical);

            if let Some(alias_list) = entry.aliases {
                for alias in alias_list {
                    let expanded_alias = expand_identifier(&alias, &document.context);
                    register_identifier(&mut aliases, &mut seen, &alias);
                    register_identifier(&mut aliases, &mut seen, &expanded_alias);
                }
            }

            let resolved = ResolvedUnit::new(
                entry.id.clone(),
                expanded_identifier.clone(),
                entry.canonical,
                entry.dimension,
                aliases.clone(),
            );

            for identifier in aliases {
                lookup.insert(identifier, resolved.clone());
            }

            entries.push(resolved);
        }

        Ok(Self { lookup, entries })
    }

    pub fn resolve_identifier(&self, identifier: &str) -> Option<ResolvedUnit> {
        self.lookup.get(identifier).cloned()
    }

    pub fn snapshot_entries(&self) -> PyResult<serde_json::Value> {
        Ok(serde_json::to_value(&self.entries)
            .expect("serializing lexicon entries to JSON value should not fail"))
    }
}

fn register_identifier(target: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
    if seen.insert(value.to_owned()) {
        target.push(value.to_owned());
    }
}

fn expand_identifier(identifier: &str, context: &HashMap<String, String>) -> String {
    if let Some((prefix, suffix)) = identifier.split_once(':') {
        if let Some(base) = context.get(prefix) {
            return format!("{base}{suffix}");
        }
    }

    identifier.to_owned()
}

#[pymethods]
impl LexiconRegistry {
    #[new]
    #[pyo3(signature = (jsonld = None))]
    fn new(jsonld: Option<String>) -> PyResult<Self> {
        let data = jsonld.as_deref().unwrap_or(DEFAULT_LEXICON);
        Self::from_jsonld_str(data)
    }

    fn resolve(&self, identifier: &str) -> PyResult<ResolvedUnit> {
        self.resolve_identifier(identifier)
            .ok_or_else(|| PyValueError::new_err(format!("unknown unit identifier: {identifier}")))
    }

    fn canonical(&self, identifier: &str) -> PyResult<String> {
        Ok(self.resolve(identifier)?.canonical)
    }

    #[pyo3(signature = (_unused = None))]
    fn known_identifiers(&self, _unused: Option<bool>) -> Vec<String> {
        let mut identifiers: Vec<String> = self.lookup.keys().cloned().collect();
        identifiers.sort();
        identifiers
    }

    #[pyo3(signature = (_unused = None))]
    fn as_json(&self, _unused: Option<bool>) -> PyResult<String> {
        Ok(serde_json::to_string_pretty(&self.entries)
            .expect("serializing lexicon entries to JSON string should not fail"))
    }
}

#[pymodule]
fn _rosettastune(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module
        .add_class::<LexiconRegistry>()
        .expect("failed to register LexiconRegistry Python class");
    module
        .add_class::<ResolvedUnit>()
        .expect("failed to register ResolvedUnit Python class");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static PYTHON_INIT: Once = Once::new();

    fn ensure_python() {
        PYTHON_INIT.call_once(pyo3::prepare_freethreaded_python);
    }

    fn minimal_lexicon() -> &'static str {
        r#"
{
  "@context": {
    "ex": "https://example.com/"
  },
  "@graph": [
    {
      "@id": "ex:kg",
      "canonical": "kg",
      "dimension": "mass",
      "aliases": ["ex:kilogram", "KGM"]
    },
    {
      "@id": "ex:m",
      "canonical": "m",
      "aliases": ["ex:meter"]
    }
  ]
}
"#
    }

    fn no_alias_lexicon() -> &'static str {
        r#"
{
    "@context": {
        "ex": "https://example.com/"
    },
    "@graph": [
        {
            "@id": "ex:s",
            "canonical": "s"
        }
    ]
}
"#
    }

    #[test]
    fn from_jsonld_str_parses_and_resolves_entries() {
        let registry = LexiconRegistry::from_jsonld_str(minimal_lexicon()).unwrap();

        let resolved = registry.resolve_identifier("ex:kilogram").unwrap();
        assert_eq!(resolved.canonical, "kg");
        assert_eq!(resolved.iri, "https://example.com/kg");
        assert_eq!(resolved.dimension.as_deref(), Some("mass"));
        assert!(resolved
            .aliases
            .contains(&"https://example.com/kilogram".to_string()));

        let resolved_meter = registry
            .resolve_identifier("https://example.com/meter")
            .unwrap();
        assert_eq!(resolved_meter.canonical, "m");
    }

    #[test]
    fn from_jsonld_str_returns_error_for_invalid_json() {
        ensure_python();
        let error = LexiconRegistry::from_jsonld_str("{ not json").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("invalid JSON-LD lexicon"));
    }

    #[test]
    fn resolve_identifier_returns_none_for_unknown_identifier() {
        let registry = LexiconRegistry::from_jsonld_str(minimal_lexicon()).unwrap();
        assert!(registry.resolve_identifier("unknown").is_none());
    }

    #[test]
    fn snapshot_entries_returns_serializable_value() {
        let registry = LexiconRegistry::from_jsonld_str(minimal_lexicon()).unwrap();
        let snapshot = registry.snapshot_entries().unwrap();
        let snapshot_text = serde_json::to_string(&snapshot).unwrap();
        assert!(snapshot_text.contains("kg"));
        assert!(snapshot_text.contains("m"));
    }

    #[test]
    fn from_jsonld_str_supports_entries_without_aliases() {
        let registry = LexiconRegistry::from_jsonld_str(no_alias_lexicon()).unwrap();
        let resolved = registry.resolve_identifier("ex:s").unwrap();
        assert_eq!(resolved.canonical, "s");
    }

    #[test]
    fn register_identifier_deduplicates_values() {
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        register_identifier(&mut items, &mut seen, "alpha");
        register_identifier(&mut items, &mut seen, "alpha");
        register_identifier(&mut items, &mut seen, "beta");

        assert_eq!(items, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn expand_identifier_handles_prefixed_and_raw_values() {
        let mut context = HashMap::new();
        context.insert("ex".to_string(), "https://example.com/".to_string());

        assert_eq!(
            expand_identifier("ex:item", &context),
            "https://example.com/item"
        );
        assert_eq!(expand_identifier("unknown:item", &context), "unknown:item");
        assert_eq!(expand_identifier("plain", &context), "plain");
    }

    #[test]
    fn pymethod_new_uses_default_lexicon_when_not_provided() {
        let registry = LexiconRegistry::new(None).unwrap();
        assert!(registry.resolve_identifier("qudt:KILO_GM").is_some());
    }

    #[test]
    fn pymethod_resolve_and_canonical_cover_success_and_error_paths() {
        ensure_python();
        let registry = LexiconRegistry::new(Some(minimal_lexicon().to_string())).unwrap();

        let resolved = registry.resolve("ex:kg").unwrap();
        assert_eq!(resolved.canonical, "kg");
        assert_eq!(registry.canonical("ex:kg").unwrap(), "kg");

        let err = registry.resolve("missing").unwrap_err();
        assert!(err.to_string().contains("unknown unit identifier"));

        let canonical_err = registry.canonical("missing").unwrap_err();
        assert!(canonical_err
            .to_string()
            .contains("unknown unit identifier"));
    }

    #[test]
    fn known_identifiers_returns_sorted_values() {
        let registry = LexiconRegistry::new(Some(minimal_lexicon().to_string())).unwrap();
        let known = registry.known_identifiers(None);

        assert_eq!(known.first().unwrap(), "KGM");
        assert!(known.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn as_json_returns_expected_content() {
        let registry = LexiconRegistry::new(Some(minimal_lexicon().to_string())).unwrap();
        let json = registry.as_json(None).unwrap();

        assert!(json.contains("\"canonical\": \"kg\""));
        assert!(json.contains("\"canonical\": \"m\""));
    }

    #[test]
    fn pymodule_registration_adds_expected_classes() {
        ensure_python();

        Python::with_gil(|py| {
            let module = PyModule::new_bound(py, "_rosettastune").unwrap();
            _rosettastune(py, &module).unwrap();

            assert!(module.hasattr("LexiconRegistry").unwrap());
            assert!(module.hasattr("ResolvedUnit").unwrap());
        });
    }

    #[test]
    fn python_calls_exercise_pymethod_wrappers() {
        ensure_python();

        Python::with_gil(|py| {
            let module = PyModule::new_bound(py, "_rosettastune").unwrap();
            _rosettastune(py, &module).unwrap();

            let cls = module.getattr("LexiconRegistry").unwrap();
            let registry = cls.call1((minimal_lexicon(),)).unwrap();

            let canonical: String = registry
                .call_method1("canonical", ("ex:kg",))
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(canonical, "kg");

            let resolved = registry.call_method1("resolve", ("ex:kg",)).unwrap();
            let resolved_canonical: String =
                resolved.getattr("canonical").unwrap().extract().unwrap();
            assert_eq!(resolved_canonical, "kg");

            let known: Vec<String> = registry
                .call_method0("known_identifiers")
                .unwrap()
                .extract()
                .unwrap();
            assert!(!known.is_empty());

            // Call descriptors directly to exercise generated no-arg wrappers.
            let known_descriptor: Vec<String> = cls
                .getattr("known_identifiers")
                .unwrap()
                .call1((registry.clone(),))
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(known_descriptor, known);

            let as_json: String = registry.call_method0("as_json").unwrap().extract().unwrap();
            assert!(as_json.contains("\"canonical\": \"kg\""));

            let as_json_descriptor: String = cls
                .getattr("as_json")
                .unwrap()
                .call1((registry.clone(),))
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(as_json_descriptor, as_json);

            let unknown = registry.call_method1("canonical", ("missing",));
            assert!(unknown.is_err());
        });
    }
}
