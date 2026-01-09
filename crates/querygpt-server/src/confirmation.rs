use crate::api_types::ConfirmAction;
use querygpt_core::planner::confirmation::{ConfirmationResult, UserConfirmation};
use querygpt_core::planner::diff::SpecDiff;

/// Server-side confirmation handler
/// This doesn't actually confirm - it signals that confirmation is needed
/// The actual confirmation happens through the /confirm endpoint
#[derive(Debug, Clone)]
pub struct ServerConfirmation {
    /// Whether to auto-approve all changes (for testing or trusted clients)
    pub auto_approve: bool,
}

impl ServerConfirmation {
    pub fn new(auto_approve: bool) -> Self {
        Self { auto_approve }
    }

    pub fn auto_approve() -> Self {
        Self { auto_approve: true }
    }

    pub fn require_approval() -> Self {
        Self {
            auto_approve: false,
        }
    }
}

impl UserConfirmation for ServerConfirmation {
    fn confirm_changes(&self, diffs: &[SpecDiff], _attempt: usize) -> ConfirmationResult {
        if self.auto_approve || diffs.is_empty() {
            ConfirmationResult::Approved
        } else {
            // Signal that we need to wait for user input
            // The server will pause here and return a PendingConfirmation response
            // This is handled specially in the orchestration flow
            ConfirmationResult::RequestRevision("PENDING_USER_CONFIRMATION".to_string())
        }
    }
}

/// Convert ConfirmAction to ConfirmationResult
impl From<ConfirmAction> for ConfirmationResult {
    fn from(action: ConfirmAction) -> Self {
        match action {
            ConfirmAction::Approve => ConfirmationResult::Approved,
            ConfirmAction::Reject => ConfirmationResult::Rejected,
            ConfirmAction::Modify { feedback } => ConfirmationResult::RequestRevision(feedback),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use querygpt_core::planner::diff::{ChangeType, SpecDiff};

    #[test]
    fn test_auto_approve_confirmation() {
        let confirmation = ServerConfirmation::auto_approve();
        let diffs = vec![SpecDiff {
            field_path: "mode".to_string(),
            change_type: ChangeType::Modified,
            old_value: Some(serde_json::Value::String("preview".to_string())),
            new_value: Some(serde_json::Value::String("export".to_string())),
        }];

        let result = confirmation.confirm_changes(&diffs, 1);
        assert_eq!(result, ConfirmationResult::Approved);
    }

    #[test]
    fn test_require_approval_with_diffs() {
        let confirmation = ServerConfirmation::require_approval();
        let diffs = vec![SpecDiff {
            field_path: "mode".to_string(),
            change_type: ChangeType::Modified,
            old_value: Some(serde_json::Value::String("preview".to_string())),
            new_value: Some(serde_json::Value::String("export".to_string())),
        }];

        let result = confirmation.confirm_changes(&diffs, 1);
        assert_eq!(
            result,
            ConfirmationResult::RequestRevision("PENDING_USER_CONFIRMATION".to_string())
        );
    }

    #[test]
    fn test_require_approval_empty_diffs() {
        let confirmation = ServerConfirmation::require_approval();
        let diffs = vec![];

        let result = confirmation.confirm_changes(&diffs, 1);
        assert_eq!(result, ConfirmationResult::Approved);
    }

    #[test]
    fn test_confirm_action_to_result() {
        let approve: ConfirmationResult = ConfirmAction::Approve.into();
        assert_eq!(approve, ConfirmationResult::Approved);

        let reject: ConfirmationResult = ConfirmAction::Reject.into();
        assert_eq!(reject, ConfirmationResult::Rejected);

        let modify: ConfirmationResult = ConfirmAction::Modify {
            feedback: "test".to_string(),
        }
        .into();
        assert_eq!(
            modify,
            ConfirmationResult::RequestRevision("test".to_string())
        );
    }
}
