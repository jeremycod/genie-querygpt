use crate::schema::cards::{SchemaCards, WorkspaceIndex};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Deserialize)]
pub struct SchemaRegistry {
    pub index: WorkspaceIndex,
    pub cards: SchemaCards,
}

impl SchemaRegistry {
    pub fn load(index_path: &str) -> anyhow::Result<Self> {
        let idx = std::fs::read_to_string(index_path)
            .with_context(|| format!("read workspace index: {}", index_path))?;
        let index: WorkspaceIndex = serde_json::from_str(&idx)?;

        // Resolve schema_cards_path relative to the repo root (two levels up from index file)
        let cards_path = if std::path::Path::new(&index.schema_cards_path).is_absolute() {
            index.schema_cards_path.clone()
        } else {
            let index_path_buf = std::path::Path::new(index_path);
            let repo_root = index_path_buf
                .parent() // config/workspaces
                .and_then(|p| p.parent()) // config
                .and_then(|p| p.parent()) // repo root
                .unwrap_or_else(|| std::path::Path::new("."));
            repo_root
                .join(&index.schema_cards_path)
                .to_str()
                .unwrap()
                .to_string()
        };

        let cards_raw = std::fs::read_to_string(&cards_path)
            .with_context(|| format!("read schema cards: {}", index.schema_cards_path))?;
        let cards: SchemaCards = serde_json::from_str(&cards_raw)?;

        Ok(Self { index, cards })
    }
}

/// Metadata about a workspace without loading full schema cards
#[derive(Debug, Clone)]
pub struct WorkspaceMetadata {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub entities: Vec<String>,
    pub index_path: PathBuf,
}

/// Multi-workspace registry with lazy loading and caching
#[derive(Debug, Clone)]
pub struct WorkspaceRegistry {
    workspaces_dir: PathBuf,
    // TODO:PERF: Consider using dashmap for concurrent access without full lock
    cache: Arc<RwLock<HashMap<String, SchemaRegistry>>>,
    metadata: Arc<HashMap<String, WorkspaceMetadata>>,
}

impl WorkspaceRegistry {
    /// Create a new registry by discovering workspaces in the given directory
    pub fn from_directory<P: AsRef<Path>>(workspaces_dir: P) -> Result<Self> {
        let workspaces_dir = workspaces_dir.as_ref().to_path_buf();

        if !workspaces_dir.exists() {
            anyhow::bail!(
                "workspaces directory does not exist: {}",
                workspaces_dir.display()
            );
        }

        let metadata = Self::discover_workspaces(&workspaces_dir)?;

        Ok(Self {
            workspaces_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(metadata),
        })
    }

    /// Discover all workspace index files in the directory
    fn discover_workspaces(dir: &Path) -> Result<HashMap<String, WorkspaceMetadata>> {
        let mut workspaces = HashMap::new();

        // Read all *.index.json files in the workspaces directory
        for entry in std::fs::read_dir(dir)
            .with_context(|| format!("failed to read workspaces directory: {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            // Look for *.index.json files
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if file_name_str.ends_with(".index.json") && path.is_file() {
                    match Self::load_workspace_metadata(&path) {
                        Ok(metadata) => {
                            workspaces.insert(metadata.name.clone(), metadata);
                        }
                        Err(e) => {
                            // TODO:ERROR: Add warning logging instead of silently skipping
                            eprintln!(
                                "Warning: failed to load workspace index {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        if workspaces.is_empty() {
            anyhow::bail!("no workspace index files found in {}", dir.display());
        }

        Ok(workspaces)
    }

    /// Load workspace metadata from an index file
    fn load_workspace_metadata(index_path: &Path) -> Result<WorkspaceMetadata> {
        let content = std::fs::read_to_string(index_path)
            .with_context(|| format!("read workspace index: {}", index_path.display()))?;

        let index: WorkspaceIndex = serde_json::from_str(&content)
            .with_context(|| format!("parse workspace index: {}", index_path.display()))?;

        Ok(WorkspaceMetadata {
            name: index.workspace.clone(),
            description: index.description,
            tags: index.tags,
            entities: index.entities,
            index_path: index_path.to_path_buf(),
        })
    }

    /// List all available workspace names
    pub fn list_workspaces(&self) -> Vec<String> {
        let mut names: Vec<String> = self.metadata.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get metadata for a specific workspace
    pub fn get_metadata(&self, workspace: &str) -> Option<&WorkspaceMetadata> {
        self.metadata.get(workspace)
    }

    /// Get all workspace metadata
    pub fn all_metadata(&self) -> Vec<&WorkspaceMetadata> {
        self.metadata.values().collect()
    }

    /// Check if a workspace exists
    pub fn has_workspace(&self, workspace: &str) -> bool {
        self.metadata.contains_key(workspace)
    }

    /// Load a workspace's full schema registry (with caching)
    pub fn load_workspace(&self, workspace: &str) -> Result<SchemaRegistry> {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(registry) = cache.get(workspace) {
                // Return a clone since SchemaRegistry implements Clone via its fields
                // TODO:PERF: Consider using Arc<SchemaRegistry> to avoid cloning
                return Ok(SchemaRegistry {
                    index: registry.index.clone(),
                    cards: registry.cards.clone(),
                });
            }
        }

        // Load from disk
        let metadata = self
            .metadata
            .get(workspace)
            .ok_or_else(|| anyhow::anyhow!("workspace not found: {}", workspace))?;

        let registry =
            SchemaRegistry::load(metadata.index_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("invalid path: {}", metadata.index_path.display())
            })?)?;

        // Cache it
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(
                workspace.to_string(),
                SchemaRegistry {
                    index: registry.index.clone(),
                    cards: registry.cards.clone(),
                },
            );
        }

        Ok(registry)
    }

    /// Preload all workspaces into cache
    /// Useful for warming up the cache at startup
    // TODO:FEATURE: Add async version for non-blocking preload
    pub fn preload_all(&self) -> Result<()> {
        for workspace in self.list_workspaces() {
            self.load_workspace(&workspace)?;
        }
        Ok(())
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Get the workspaces directory path
    pub fn workspaces_dir(&self) -> &Path {
        &self.workspaces_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_registry_discovery() {
        // Test with the actual config/workspaces directory
        let workspaces_dir = PathBuf::from("config/workspaces");

        if !workspaces_dir.exists() {
            // Skip test if running in an environment without the config directory
            eprintln!("Skipping test: config/workspaces directory not found");
            return;
        }

        let registry =
            WorkspaceRegistry::from_directory(&workspaces_dir).expect("failed to create registry");

        // Should discover at least the campaigns_offers workspace
        let workspaces = registry.list_workspaces();
        assert!(
            !workspaces.is_empty(),
            "should discover at least one workspace"
        );
        assert!(
            workspaces.contains(&"campaigns_offers".to_string()),
            "should discover campaigns_offers workspace"
        );

        // Should have metadata for discovered workspaces
        for workspace in &workspaces {
            let metadata = registry.get_metadata(workspace);
            assert!(metadata.is_some(), "should have metadata for {}", workspace);
        }
    }

    #[test]
    fn test_workspace_loading_and_caching() {
        let workspaces_dir = PathBuf::from("config/workspaces");

        if !workspaces_dir.exists() {
            eprintln!("Skipping test: config/workspaces directory not found");
            return;
        }

        let registry =
            WorkspaceRegistry::from_directory(&workspaces_dir).expect("failed to create registry");

        // Load a workspace
        let schema = registry
            .load_workspace("campaigns_offers")
            .expect("failed to load campaigns_offers workspace");

        assert_eq!(schema.index.workspace, "campaigns_offers");
        assert!(!schema.cards.entities.is_empty());

        // Load again - should come from cache
        let schema2 = registry
            .load_workspace("campaigns_offers")
            .expect("failed to load campaigns_offers workspace second time");

        assert_eq!(schema2.index.workspace, "campaigns_offers");
    }

    #[test]
    fn test_workspace_not_found() {
        let workspaces_dir = PathBuf::from("config/workspaces");

        if !workspaces_dir.exists() {
            eprintln!("Skipping test: config/workspaces directory not found");
            return;
        }

        let registry =
            WorkspaceRegistry::from_directory(&workspaces_dir).expect("failed to create registry");

        // Try to load non-existent workspace
        let result = registry.load_workspace("nonexistent_workspace");
        assert!(
            result.is_err(),
            "should fail to load non-existent workspace"
        );
    }

    #[test]
    fn test_has_workspace() {
        let workspaces_dir = PathBuf::from("config/workspaces");

        if !workspaces_dir.exists() {
            eprintln!("Skipping test: config/workspaces directory not found");
            return;
        }

        let registry =
            WorkspaceRegistry::from_directory(&workspaces_dir).expect("failed to create registry");

        assert!(registry.has_workspace("campaigns_offers"));
        assert!(!registry.has_workspace("nonexistent"));
    }

    #[test]
    fn test_pricing_workspace_discovery() {
        let workspaces_dir = PathBuf::from("config/workspaces");

        if !workspaces_dir.exists() {
            eprintln!("Skipping test: config/workspaces directory not found");
            return;
        }

        let registry =
            WorkspaceRegistry::from_directory(&workspaces_dir).expect("failed to create registry");

        let workspaces = registry.list_workspaces();
        assert!(
            workspaces.contains(&"pricing_discounts".to_string()),
            "pricing_discounts workspace should be discovered"
        );

        let metadata = registry
            .get_metadata("pricing_discounts")
            .expect("pricing_discounts metadata should exist");

        assert_eq!(metadata.name, "pricing_discounts");
        assert!(metadata.tags.contains(&"pricing".to_string()));
        assert!(metadata.entities.contains(&"products_latest".to_string()));

        // Test loading the workspace
        let schema = registry
            .load_workspace("pricing_discounts")
            .expect("should load pricing_discounts workspace");

        assert_eq!(schema.cards.workspace, "pricing_discounts");
        assert_eq!(schema.cards.entities.len(), 4); // products, prices, discounts, offers
        assert_eq!(schema.cards.join_graph.edges.len(), 2); // products-prices, offers-discounts
    }

    #[test]
    fn test_distribution_workspace_discovery() {
        let workspaces_dir = PathBuf::from("config/workspaces");

        if !workspaces_dir.exists() {
            eprintln!("Skipping test: config/workspaces directory not found");
            return;
        }

        let registry =
            WorkspaceRegistry::from_directory(&workspaces_dir).expect("failed to create registry");

        let workspaces = registry.list_workspaces();
        assert!(
            workspaces.contains(&"distribution".to_string()),
            "distribution workspace should be discovered"
        );

        let metadata = registry
            .get_metadata("distribution")
            .expect("distribution metadata should exist");

        assert_eq!(metadata.name, "distribution");
        assert!(metadata.tags.contains(&"partners".to_string()));
        assert!(metadata.tags.contains(&"skus".to_string()));
        assert!(metadata.entities.contains(&"skus_latest".to_string()));
        assert!(metadata.entities.contains(&"partners".to_string()));

        // Test loading the workspace
        let schema = registry
            .load_workspace("distribution")
            .expect("should load distribution workspace");

        assert_eq!(schema.cards.workspace, "distribution");
        assert_eq!(schema.cards.entities.len(), 5); // skus, partners, campaigns, campaign_offers, offers
        assert_eq!(schema.cards.join_graph.edges.len(), 3); // campaigns-partners, campaigns-campaign_offers, campaign_offers-offers
    }
}
