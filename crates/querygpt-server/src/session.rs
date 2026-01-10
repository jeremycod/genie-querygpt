use querygpt_core::planner::planner::PlannerContext;
use querygpt_core::planner::trace::PlannerTrace;
use querygpt_core::schema::registry::SchemaRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Session state for multi-step query workflows
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Unique session identifier
    pub id: String,
    /// Original user prompt
    pub prompt: String,
    /// Workspace being queried
    pub workspace: String,
    /// Schema registry for compilation
    pub registry: Arc<SchemaRegistry>,
    /// Planner context
    pub context: PlannerContext,
    /// Current attempt number
    pub attempt: usize,
    /// Planner trace
    pub trace: PlannerTrace,
    /// When this session was created
    pub created_at: Instant,
    /// When this session was last accessed
    pub last_accessed: Instant,
}

impl SessionState {
    pub fn new(
        id: String,
        prompt: String,
        workspace: String,
        registry: Arc<SchemaRegistry>,
        context: PlannerContext,
        trace: PlannerTrace,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            prompt,
            workspace,
            registry,
            context,
            attempt: 1,
            trace,
            created_at: now,
            last_accessed: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }
}

/// Thread-safe session store
#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
    timeout: Duration,
}

impl SessionStore {
    /// Create a new session store with the given timeout
    pub fn new(timeout: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            timeout,
        }
    }

    /// Create a new session store with default 30-minute timeout
    pub fn with_default_timeout() -> Self {
        Self::new(Duration::from_secs(30 * 60))
    }

    /// Insert a new session
    pub async fn insert(&self, session: SessionState) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session);
    }

    /// Get a session by ID, updating last_accessed time
    pub async fn get(&self, id: &str) -> Option<SessionState> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(id) {
            if session.is_expired(self.timeout) {
                // Remove expired session
                sessions.remove(id);
                None
            } else {
                session.touch();
                Some(session.clone())
            }
        } else {
            None
        }
    }

    /// Remove a session by ID
    pub async fn remove(&self, id: &str) -> Option<SessionState> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id)
    }

    /// Remove all expired sessions
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before_count = sessions.len();

        sessions.retain(|_, session| !session.is_expired(self.timeout));

        before_count - sessions.len()
    }

    /// Get the number of active sessions
    pub async fn len(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Check if the store is empty
    pub async fn is_empty(&self) -> bool {
        let sessions = self.sessions.read().await;
        sessions.is_empty()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::with_default_timeout()
    }
}

/// Generate a unique session ID
pub fn generate_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    format!("session_{}_{}", timestamp, counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use querygpt_core::planner::schema_summary::SchemaSummary;

    fn create_test_session(id: &str) -> SessionState {
        // Use absolute path to avoid working directory issues with concurrent tests
        let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = crate_root
            .parent()
            .and_then(|p| p.parent())
            .expect("resolve repo root from CARGO_MANIFEST_DIR");
        let index_path = repo_root.join("config/workspaces/campaigns_offers.index.json");

        let registry = Arc::new(
            SchemaRegistry::load(index_path.to_str().unwrap()).expect("load test registry"),
        );
        let context = PlannerContext::enhanced(
            "test".to_string(),
            SchemaSummary::minimal("test"),
            vec![],
            None,
        );
        let trace = PlannerTrace::new("test".to_string());

        SessionState::new(
            id.to_string(),
            "test prompt".to_string(),
            "test".to_string(),
            registry,
            context,
            trace,
        )
    }

    #[tokio::test]
    async fn test_session_store_insert_and_get() {
        let store = SessionStore::new(Duration::from_secs(60));
        let session = create_test_session("test_id");

        store.insert(session.clone()).await;

        let retrieved = store.get("test_id").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test_id");
    }

    #[tokio::test]
    async fn test_session_store_remove() {
        let store = SessionStore::new(Duration::from_secs(60));
        let session = create_test_session("test_id");

        store.insert(session).await;
        assert!(store.get("test_id").await.is_some());

        store.remove("test_id").await;
        assert!(store.get("test_id").await.is_none());
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let store = SessionStore::new(Duration::from_millis(100));
        let session = create_test_session("test_id");

        store.insert(session).await;
        assert!(store.get("test_id").await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired and removed
        assert!(store.get("test_id").await.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let store = SessionStore::new(Duration::from_millis(100));

        // Insert multiple sessions
        for i in 0..5 {
            let session = create_test_session(&format!("test_{}", i));
            store.insert(session).await;
        }

        assert_eq!(store.len().await, 5);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Cleanup
        let removed = store.cleanup_expired().await;
        assert_eq!(removed, 5);
        assert_eq!(store.len().await, 0);
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("session_"));
        assert!(id2.starts_with("session_"));
    }
}
