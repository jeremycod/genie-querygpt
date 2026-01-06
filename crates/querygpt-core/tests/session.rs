#[cfg(test)]
mod tests {
    use querygpt_core::dsl::report_spec::ReportSpec;
    use querygpt_core::planner::session::PlannerSession;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn make_spec(workspace: &str, field: &str, limit: i64) -> ReportSpec {
        ReportSpec {
            version: 1,
            workspace: workspace.to_string(),
            select: vec![querygpt_core::dsl::report_spec::SelectItem {
                field: field.to_string(),
                alias: None,
            }],
            filters: vec![],
            order_by: vec![],
            mode: querygpt_core::dsl::report_spec::Mode::Preview,
            pagination: Some(querygpt_core::dsl::report_spec::PaginationSpec {
                limit: Some(limit),
                offset: None,
            }),
        }
    }

    #[test]
    fn refuses_without_confirmation() {
        let compile_calls = Rc::new(RefCell::new(0));
        let compile_calls_clone = compile_calls.clone();
        let compiler = move |_spec: &ReportSpec| {
            *compile_calls_clone.borrow_mut() += 1;
            Ok(format!("plan-{}", *compile_calls_clone.borrow()))
        };

        let user_spec = make_spec("ws", "a", 10);
        let suggested_spec = make_spec("ws", "b", 10);
        let session = PlannerSession::new("show me stuff", user_spec, suggested_spec, compiler)
            .expect("session should initialize");

        assert_eq!(*compile_calls.borrow(), 1, "compiler should run on init");
        assert!(session.runnable_spec(false).is_err());
        assert_eq!(session.runnable_spec(true).unwrap().select[0].field, "b");
        assert!(session.diff().has_changes());
    }

    #[test]
    fn recompile_on_any_spec_change() {
        let compile_calls = Rc::new(RefCell::new(0));
        let compile_calls_clone = compile_calls.clone();
        let compiler = move |spec: &ReportSpec| {
            *compile_calls_clone.borrow_mut() += 1;
            Ok(format!(
                "plan:{}:{}",
                spec.workspace,
                *compile_calls_clone.borrow()
            ))
        };

        let user_spec = make_spec("ws", "a", 10);
        let suggested_spec = make_spec("ws", "b", 10);
        let mut session = PlannerSession::new("show me stuff", user_spec, suggested_spec, compiler)
            .expect("session should initialize");

        assert_eq!(*compile_calls.borrow(), 1, "compiler should run on init");

        session
            .update_suggested_spec(make_spec("ws", "c", 20))
            .expect("update succeeds");
        assert_eq!(*compile_calls.borrow(), 2, "compiler reruns on change");

        session
            .update_suggested_spec(make_spec("ws", "d", 30))
            .expect("second change succeeds");
        assert_eq!(
            *compile_calls.borrow(),
            3,
            "compiler reruns on every change"
        );
    }
}
