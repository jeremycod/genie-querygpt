use crate::dsl::report_spec::ReportSpec;
use serde::Serialize;
use serde_json;

/// Represents a change between two ReportSpec versions
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecDiff {
    pub field_path: String,
    pub change_type: ChangeType,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
}

/// Generate structured diff between two ReportSpecs
pub fn diff_report_specs(original: &ReportSpec, revised: &ReportSpec) -> Vec<SpecDiff> {
    let mut diffs = Vec::new();

    let original_json = serde_json::to_value(original).unwrap();
    let revised_json = serde_json::to_value(revised).unwrap();

    compare_json_values("", &original_json, &revised_json, &mut diffs);

    diffs
}

fn compare_json_values(
    path: &str,
    original: &serde_json::Value,
    revised: &serde_json::Value,
    diffs: &mut Vec<SpecDiff>,
) {
    match (original, revised) {
        (serde_json::Value::Object(orig_map), serde_json::Value::Object(rev_map)) => {
            // Check for removed or modified fields
            for (key, orig_val) in orig_map {
                let field_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                match rev_map.get(key) {
                    Some(rev_val) => {
                        if orig_val != rev_val {
                            compare_json_values(&field_path, orig_val, rev_val, diffs);
                        }
                    }
                    None => {
                        diffs.push(SpecDiff {
                            field_path,
                            change_type: ChangeType::Removed,
                            old_value: Some(orig_val.clone()),
                            new_value: None,
                        });
                    }
                }
            }

            // Check for added fields
            for (key, rev_val) in rev_map {
                if !orig_map.contains_key(key) {
                    let field_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    diffs.push(SpecDiff {
                        field_path,
                        change_type: ChangeType::Added,
                        old_value: None,
                        new_value: Some(rev_val.clone()),
                    });
                }
            }
        }
        (serde_json::Value::Array(orig_arr), serde_json::Value::Array(rev_arr)) => {
            if orig_arr != rev_arr {
                diffs.push(SpecDiff {
                    field_path: path.to_string(),
                    change_type: ChangeType::Modified,
                    old_value: Some(serde_json::Value::Array(orig_arr.clone())),
                    new_value: Some(serde_json::Value::Array(rev_arr.clone())),
                });
            }
        }
        _ => {
            if original != revised {
                diffs.push(SpecDiff {
                    field_path: path.to_string(),
                    change_type: ChangeType::Modified,
                    old_value: Some(original.clone()),
                    new_value: Some(revised.clone()),
                });
            }
        }
    }
}

/// Format diff for display to user
pub fn format_diff_display(diffs: &[SpecDiff]) -> String {
    if diffs.is_empty() {
        return "No changes detected.".to_string();
    }

    let mut output = String::new();
    output.push_str("Changes to ReportSpec:\n");
    output.push_str("=====================\n\n");

    for diff in diffs {
        match diff.change_type {
            ChangeType::Added => {
                output.push_str(&format!(
                    "+ Added {}: {}\n",
                    diff.field_path,
                    format_value(&diff.new_value)
                ));
            }
            ChangeType::Removed => {
                output.push_str(&format!(
                    "- Removed {}: {}\n",
                    diff.field_path,
                    format_value(&diff.old_value)
                ));
            }
            ChangeType::Modified => {
                output.push_str(&format!("~ Modified {}:\n", diff.field_path));
                output.push_str(&format!("  - {}\n", format_value(&diff.old_value)));
                output.push_str(&format!("  + {}\n", format_value(&diff.new_value)));
            }
        }
        output.push('\n');
    }

    output
}

fn format_value(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(v) => match v {
            serde_json::Value::String(s) => format!("\"{}\"", s),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
            serde_json::Value::Object(_) => "{object}".to_string(),
        },
        None => "(none)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::report_spec::{Mode, SelectItem};

    #[test]
    fn diff_detects_no_changes() {
        let spec = ReportSpec {
            version: 1,
            workspace: "test".to_string(),
            select: vec![SelectItem {
                field: "field1".to_string(),
                alias: None,
            }],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        };

        let diffs = diff_report_specs(&spec, &spec);
        assert!(diffs.is_empty());
    }

    #[test]
    fn diff_detects_field_changes() {
        let original = ReportSpec {
            version: 1,
            workspace: "test".to_string(),
            select: vec![SelectItem {
                field: "field1".to_string(),
                alias: None,
            }],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        };

        let revised = ReportSpec {
            version: 1,
            workspace: "test".to_string(),
            select: vec![SelectItem {
                field: "field2".to_string(),
                alias: None,
            }],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Export,
            pagination: None,
        };

        let diffs = diff_report_specs(&original, &revised);
        assert!(!diffs.is_empty());

        // Should detect mode change and select field change
        let mode_diff = diffs.iter().find(|d| d.field_path == "mode");
        assert!(mode_diff.is_some());
        assert_eq!(mode_diff.unwrap().change_type, ChangeType::Modified);
    }

    #[test]
    fn format_diff_display_works() {
        let diffs = vec![SpecDiff {
            field_path: "mode".to_string(),
            change_type: ChangeType::Modified,
            old_value: Some(serde_json::Value::String("preview".to_string())),
            new_value: Some(serde_json::Value::String("export".to_string())),
        }];

        let display = format_diff_display(&diffs);
        assert!(display.contains("Changes to ReportSpec"));
        assert!(display.contains("Modified mode"));
        assert!(display.contains("preview"));
        assert!(display.contains("export"));
    }
}
