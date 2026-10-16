//! The tiered runner catalog.

use crate::defaults;
use crate::definition::RunnerDefinition;
use crate::error::{Error, Result};

/// Where a declaration came from, in descending order of trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// Compiled into this binary; identifiers are reserved.
    Compiled,
    /// Supplied by a catalog file named in trusted launch configuration.
    ConfigFile,
    /// Supplied by the caller in a task document or control frame.
    Caller,
}

/// One catalog entry with its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The declaration itself.
    pub definition: RunnerDefinition,
    /// Which tier supplied it.
    pub tier: Tier,
}

/// A resolved set of runner declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Catalog {
    entries: Vec<Entry>,
}

impl Catalog {
    /// A catalog holding only the compiled-in tier.
    pub fn built_in() -> Result<Self> {
        let mut catalog = Self::empty();
        catalog.extend(Tier::Compiled, defaults::built_in()?)?;
        Ok(catalog)
    }

    /// A catalog holding nothing.
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add declarations from one tier.
    ///
    /// No tier may claim an identifier already present. A compiled-in identifier
    /// is reported as reserved and any other collision as a duplicate, so an
    /// operator who needs different launch arguments declares a differently named
    /// runner rather than silently replacing one. Refusing the whole batch rather
    /// than skipping the collision keeps a malformed catalog from producing a
    /// quietly different runner set.
    pub fn extend(&mut self, tier: Tier, definitions: Vec<RunnerDefinition>) -> Result<()> {
        {
            let mut seen: Vec<&str> = Vec::new();
            for definition in &definitions {
                if let Some(existing) = self
                    .entries
                    .iter()
                    .find(|e| e.definition.id == definition.id)
                {
                    return Err(match existing.tier {
                        Tier::Compiled => Error::Reserved(definition.id.clone()),
                        _ => Error::Duplicate(definition.id.clone()),
                    });
                }
                if seen.contains(&definition.id.as_str()) {
                    return Err(Error::Duplicate(definition.id.clone()));
                }
                seen.push(&definition.id);
            }
        }
        self.entries.extend(
            definitions
                .into_iter()
                .map(|definition| Entry { definition, tier }),
        );
        Ok(())
    }

    /// Resolve an identifier to an enabled declaration.
    ///
    /// An unknown identifier and a disabled one are distinct errors, and neither
    /// falls back to a default runner.
    pub fn resolve(&self, id: &str) -> Result<&RunnerDefinition> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.definition.id == id)
            .ok_or_else(|| Error::Unknown(id.to_owned()))?;
        if !entry.definition.enabled {
            return Err(Error::Disabled(id.to_owned()));
        }
        Ok(&entry.definition)
    }

    /// Every declaration, including disabled ones, in insertion order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Only the enabled declarations, in insertion order.
    pub fn list(&self) -> Vec<&RunnerDefinition> {
        self.entries
            .iter()
            .filter(|entry| entry.definition.enabled)
            .map(|entry| &entry.definition)
            .collect()
    }

    /// The tier that supplied an identifier, when present.
    pub fn tier_of(&self, id: &str) -> Option<Tier> {
        self.entries
            .iter()
            .find(|entry| entry.definition.id == id)
            .map(|entry| entry.tier)
    }
}
