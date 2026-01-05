use crate::planner::diff::{SpecDiff, format_diff_display};
use std::io::{self, Write};

/// User confirmation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmationResult {
    Approved,
    Rejected,
    RequestRevision(String), // User feedback for revision
}

/// Trait for user confirmation interface
pub trait UserConfirmation {
    fn confirm_changes(&self, diffs: &[SpecDiff], attempt: usize) -> ConfirmationResult;
}

/// Interactive console confirmation - prompts user for input
#[derive(Debug, Default)]
pub struct InteractiveConfirmation;

impl UserConfirmation for InteractiveConfirmation {
    fn confirm_changes(&self, diffs: &[SpecDiff], attempt: usize) -> ConfirmationResult {
        if diffs.is_empty() {
            return ConfirmationResult::Approved;
        }
        
        println!("{}", format_confirmation_prompt(diffs, attempt));
        
        loop {
            print!("Your choice [A/R/M]: ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let choice = input.trim().to_lowercase();
                    match choice.as_str() {
                        "a" | "approve" => return ConfirmationResult::Approved,
                        "r" | "reject" => return ConfirmationResult::Rejected,
                        "m" | "modify" => {
                            print!("Enter modification request: ");
                            io::stdout().flush().unwrap();
                            let mut feedback = String::new();
                            if io::stdin().read_line(&mut feedback).is_ok() {
                                return ConfirmationResult::RequestRevision(feedback.trim().to_string());
                            }
                        }
                        _ => {
                            println!("Invalid choice. Please enter A, R, or M.");
                            continue;
                        }
                    }
                }
                Err(_) => {
                    println!("Error reading input. Defaulting to reject.");
                    return ConfirmationResult::Rejected;
                }
            }
        }
    }
}

/// Mock confirmation for testing - always approves
#[derive(Debug, Default)]
pub struct MockConfirmation {
    pub should_approve: bool,
}

impl UserConfirmation for MockConfirmation {
    fn confirm_changes(&self, _diffs: &[SpecDiff], _attempt: usize) -> ConfirmationResult {
        if self.should_approve {
            ConfirmationResult::Approved
        } else {
            ConfirmationResult::Rejected
        }
    }
}

/// Auto-approve confirmation for compile-only flow
#[derive(Debug, Default)]
pub struct AutoApproveConfirmation;

impl UserConfirmation for AutoApproveConfirmation {
    fn confirm_changes(&self, _diffs: &[SpecDiff], _attempt: usize) -> ConfirmationResult {
        ConfirmationResult::Approved
    }
}

/// Console-based confirmation (for future CLI integration)
#[derive(Debug, Default)]
pub struct ConsoleConfirmation;

impl UserConfirmation for ConsoleConfirmation {
    fn confirm_changes(&self, diffs: &[SpecDiff], attempt: usize) -> ConfirmationResult {
        // In a real implementation, this would prompt the user via console
        // For now, we'll simulate based on attempt number
        if diffs.is_empty() {
            return ConfirmationResult::Approved;
        }
        
        // Simulate user behavior: approve on first attempt, reject on subsequent
        if attempt == 1 {
            ConfirmationResult::Approved
        } else {
            ConfirmationResult::Rejected
        }
    }
}

/// Format confirmation prompt for display
pub fn format_confirmation_prompt(diffs: &[SpecDiff], attempt: usize) -> String {
    let mut prompt = String::new();
    
    if attempt > 1 {
        prompt.push_str(&format!("Retry attempt #{}\n\n", attempt));
    }
    
    prompt.push_str(&format_diff_display(diffs));
    prompt.push_str("\nDo you want to proceed with these changes?\n");
    prompt.push_str("Options:\n");
    prompt.push_str("  [A]pprove - Accept the changes\n");
    prompt.push_str("  [R]eject  - Reject the changes\n");
    prompt.push_str("  [M]odify  - Request modifications\n");
    
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::diff::{SpecDiff, ChangeType};

    #[test]
    fn mock_confirmation_approves_when_configured() {
        let confirmation = MockConfirmation { should_approve: true };
        let result = confirmation.confirm_changes(&[], 1);
        assert_eq!(result, ConfirmationResult::Approved);
    }

    #[test]
    fn mock_confirmation_rejects_when_configured() {
        let confirmation = MockConfirmation { should_approve: false };
        let result = confirmation.confirm_changes(&[], 1);
        assert_eq!(result, ConfirmationResult::Rejected);
    }

    #[test]
    fn auto_approve_always_approves() {
        let confirmation = AutoApproveConfirmation;
        let result = confirmation.confirm_changes(&[], 1);
        assert_eq!(result, ConfirmationResult::Approved);
    }

    #[test]
    fn format_confirmation_prompt_includes_attempt_number() {
        let diffs = vec![
            SpecDiff {
                field_path: "mode".to_string(),
                change_type: ChangeType::Modified,
                old_value: Some(serde_json::Value::String("preview".to_string())),
                new_value: Some(serde_json::Value::String("export".to_string())),
            }
        ];
        
        let prompt = format_confirmation_prompt(&diffs, 2);
        assert!(prompt.contains("Retry attempt #2"));
        assert!(prompt.contains("Do you want to proceed"));
    }
}