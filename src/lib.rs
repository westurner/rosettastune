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
        serde_json::to_value(&self.entries)
            .map_err(|error| PyValueError::new_err(format!("failed to serialize lexicon: {error}")))
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

    fn known_identifiers(&self) -> Vec<String> {
        let mut identifiers: Vec<String> = self.lookup.keys().cloned().collect();
        identifiers.sort();
        identifiers
    }

    fn as_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.entries)
            .map_err(|error| PyValueError::new_err(format!("failed to serialize lexicon: {error}")))
    }
}

#[pymodule]
fn _rosettastune(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<LexiconRegistry>()?;
    module.add_class::<ResolvedUnit>()?;
    Ok(())
}
