use std::sync::atomic::{AtomicBool, Ordering};

use ontoforge_expr::{ExprError, MAX_EXPR_CHARS, analyze};

/// `n` layers of parentheses around `1`.
fn nested(n: usize) -> String {
    format!("{}1{}", "(".repeat(n), ")".repeat(n))
}

/// Runs `analyze` and reports whether the closure was invoked.
fn run(src: &str) -> (Result<(), ExprError>, bool) {
    let called = AtomicBool::new(false);
    let result = analyze(src, |_| called.store(true, Ordering::SeqCst));
    (result, called.load(Ordering::SeqCst))
}

#[test]
fn too_long_expression_is_rejected_without_compiling() {
    let (result, called) = run(&"a".repeat(MAX_EXPR_CHARS + 1));
    assert_eq!(
        result,
        Err(ExprError::TooLong {
            chars: MAX_EXPR_CHARS + 1
        })
    );
    assert!(!called);
}

#[test]
fn max_length_expression_is_accepted() {
    let (result, called) = run(&"a".repeat(MAX_EXPR_CHARS));
    assert_eq!(result, Ok(()));
    assert!(called);
}

#[test]
fn brackets_inside_strings_do_not_count() {
    let parens = "(".repeat(40);
    for src in [
        format!("'{parens}' == params.a"),
        format!("\"{parens}\" == params.a"),
    ] {
        let (result, called) = run(&src);
        assert_eq!(result, Ok(()), "{src}");
        assert!(called, "{src}");
    }
}

#[test]
fn triple_quoted_and_raw_strings_follow_cel_lexing() {
    let parens = "(".repeat(40);
    let triple = format!("'''it's {parens}''' == params.a");
    let (result, called) = run(&triple);
    assert_eq!(result, Ok(()), "{triple}");
    assert!(called);

    // Raw strings do not escape the closing quote, so `r'\'` is complete and the
    // 33 layers after it are real nesting.
    let raw = format!("r'\\' == {}", nested(33));
    let (result, called) = run(&raw);
    assert!(
        matches!(result, Err(ExprError::TooDeep { .. })),
        "{result:?}"
    );
    assert!(!called);

    let bytes_raw = format!("br'\\' == {}", nested(33));
    assert!(matches!(run(&bytes_raw).0, Err(ExprError::TooDeep { .. })));
}

#[test]
fn raw_prefix_requires_a_token_boundary() {
    // `ar'` is the identifier `ar` followed by an ordinary string, so `\'` escapes
    // the quote and the string swallows everything up to the final quote.
    let (result, _) = run(&format!("ar'\\' {} '", "(".repeat(40)));
    assert!(
        !matches!(result, Err(ExprError::TooDeep { .. })),
        "{result:?}"
    );

    // After a number a new token starts, so `r'\'` is raw again.
    let src = format!("1 r'\\' {}", nested(33));
    assert!(matches!(run(&src).0, Err(ExprError::TooDeep { .. })));
    let src = format!("1r'\\' {}", nested(33));
    assert!(matches!(run(&src).0, Err(ExprError::TooDeep { .. })));
}

#[test]
fn comments_do_not_open_strings() {
    let src = format!("// '''\n{}\n// '''", nested(33));
    let (result, called) = run(&src);
    assert!(
        matches!(result, Err(ExprError::TooDeep { .. })),
        "{result:?}"
    );
    assert!(!called);
}

#[test]
fn invalid_escape_is_rejected_without_compiling() {
    let (result, called) = run(&format!("'\\q{}'", "(".repeat(40)));
    assert!(
        matches!(
            result,
            Err(ExprError::Syntax {
                line: 1,
                column: 2,
                ..
            })
        ),
        "{result:?}"
    );
    assert!(!called);
}

#[test]
fn unterminated_string_is_rejected_without_compiling() {
    let (result, called) = run("params.a == 'abc");
    match result {
        Err(ExprError::Syntax {
            line,
            column,
            message,
        }) => {
            assert_eq!((line, column), (1, 13));
            assert!(message.contains("unterminated"), "{message}");
        }
        other => panic!("expected Syntax, got {other:?}"),
    }
    assert!(!called);
}

#[test]
fn deeply_nested_expression_is_rejected_not_crashing() {
    let handle = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let (too_deep, called_deep) = run(&nested(33));
            assert!(
                matches!(too_deep, Err(ExprError::TooDeep { .. })),
                "{too_deep:?}"
            );
            assert!(!called_deep);

            let (ok, called_ok) = run(&nested(31));
            assert_eq!(ok, Ok(()));
            assert!(called_ok);
        })
        .expect("spawn small-stack thread");
    handle
        .join()
        .expect("small-stack thread must not panic or overflow");
}

#[test]
fn syntax_error_has_expression_position() {
    let (result, called) = run("params.x +");
    match result {
        Err(ExprError::Syntax { line, column, .. }) => assert_eq!((line, column), (1, 11)),
        other => panic!("expected Syntax, got {other:?}"),
    }
    assert!(!called);
}

#[test]
fn closure_panic_becomes_internal() {
    let result: Result<(), ExprError> = analyze("1", |_| panic!("boom"));
    assert_eq!(result, Err(ExprError::Internal));
}

#[test]
fn flat_recursive_forms_do_not_crash_on_a_small_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            // Neither form uses brackets, so only the compiler's own limits stop them.
            for src in [
                format!("{}true", "!".repeat(4000)),
                format!("{}1", "a ? b : ".repeat(500)),
                format!("1{}", " + 1".repeat(1000)),
            ] {
                let (result, _) = run(&src);
                assert!(
                    matches!(result, Ok(()) | Err(ExprError::Syntax { .. })),
                    "{result:?}"
                );
            }
        })
        .expect("spawn small-stack thread");
    handle
        .join()
        .expect("small-stack thread must not panic or overflow");
}
