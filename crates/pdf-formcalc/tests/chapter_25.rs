//! The examples of XFA 3.3 chapter 25 (pages 1060–1149), each run through the evaluator, with
//! the page its expected value is derived from beside it (ADR 2482). The chapter is cited and
//! paraphrased; the scripts below are written out here, not copied as a listing.

#![expect(
    clippy::panic,
    reason = "a test asserts, and a failed assertion is its report"
)]

use std::collections::BTreeMap;

use pdf_formcalc::{Budget, Host, Segment, Stop, Value, run};

/// A form of named values: a path `A.B` is the key `A.B`, an occurrence `[n]` is part of the key,
/// and `[*]` is every key that begins with the name and `[`.
#[derive(Default)]
struct Form {
    fields: BTreeMap<String, Value>,
}

impl Form {
    fn with(fields: &[(&str, Value)]) -> Self {
        Self {
            fields: fields
                .iter()
                .map(|(name, value)| ((*name).to_owned(), value.clone()))
                .collect(),
        }
    }
}

impl Host for Form {
    type Node = String;

    fn resolve(&self, path: &[Segment]) -> Result<Vec<String>, String> {
        use pdf_formcalc::Occurrence;
        let name = path
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let last = path.last().ok_or("an empty path")?;
        Ok(match last.occurrence {
            Occurrence::Every => self
                .fields
                .keys()
                .filter(|key| key.starts_with(&format!("{name}[")))
                .cloned()
                .collect(),
            Occurrence::At(at) => {
                let key = format!("{name}[{at}]");
                if self.fields.contains_key(&key) {
                    vec![key]
                } else {
                    vec![]
                }
            }
            _ => {
                if self.fields.contains_key(&name) {
                    vec![name]
                } else {
                    vec![]
                }
            }
        })
    }

    fn value(&self, node: &String) -> Value {
        self.fields.get(node).cloned().unwrap_or(Value::Null)
    }

    fn assign(&mut self, node: &String, value: Value) -> Result<(), String> {
        self.fields.insert(node.clone(), value);
        Ok(())
    }
}

fn eval(source: &str) -> Result<Value, Stop> {
    run(source, &mut Form::default(), Budget::CALCULATION)
}

fn number(source: &str) -> f64 {
    match eval(source) {
        Ok(Value::Number(n)) => n,
        other => panic!("{source:?} gave {other:?}, not a number"),
    }
}

fn text(source: &str) -> String {
    match eval(source) {
        Ok(Value::String(s)) => s,
        other => panic!("{source:?} gave {other:?}, not a string"),
    }
}

#[test]
fn an_expression_list_is_worth_its_last_expression() {
    // Page 1060 (Example 25.5): the list's value is the last expression's, here 10 * 3 + 5 * 4.
    assert!((number(r#"5 + 1 "Hello World" 10 * 3 + 5 * 4"#) - 50.0).abs() < 1e-9);
}

#[test]
fn a_function_the_subset_does_not_carry_is_refused_by_name_and_page() {
    // Example 25.5 calls Abs (page 1081), which is outside the census's subset.
    match eval(r#"5 + Abs(1) "Hello World" 10 * 3 + 5 * 4"#) {
        Err(Stop::Unsupported(what)) => {
            assert!(what.contains("Abs") && what.contains("1081"), "{what}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn operands_are_promoted_as_page_1061_says() {
    // Page 1061: a string that is no number becomes 0 in arithmetic, a null becomes 0, and a
    // string that is a number becomes that number.
    assert!((number(r#"(5 - "abc") * 3"#) - 15.0).abs() < 1e-9);
    assert!((number(r#""100" / 10"#) - 10.0).abs() < 1e-9);
    assert!((number("5 + null + 3") - 8.0).abs() < 1e-9);
    assert!((number(r#"if ("abc") then 10 else 20 endif"#) - 20.0).abs() < 1e-9);
}

#[test]
fn a_division_by_zero_ends_the_script_with_an_error() {
    // Page 1062: an intermediate infinity or NaN raises an error exception.
    assert!(matches!(eval("3 / 0 + 1"), Err(Stop::Exception(_))));
}

#[test]
fn a_function_is_declared_and_invoked() {
    // Page 1066 (Example 25.13).
    assert!(
        (number("func MyFunction(param1) do param1*param1 endfunc MyFunction(3)") - 9.0).abs()
            < 1e-9
    );
}

#[test]
fn a_for_loop_sums_a_series() {
    // Page 1073 (Example 25.28): the even numbers from 2 to 100 sum to 2550.
    let sum =
        number("var x var evensum=0 for x=2 upto 100 step 2 do evensum=evensum+x endfor evensum");
    assert!((sum - 2550.0).abs() < 1e-9);
}

#[test]
fn a_block_bounds_a_declaration() {
    // Page 1075: the redeclaration inside the block ends with it, so xxx is 3 again.
    assert!((number("var xxx=3 do var xxx=1 end xxx") - 3.0).abs() < 1e-9);
}

/// Example 25.31's script (page 1075), with its comments replaced by this test's own.
const SPANS: &str = "
D = 0
var A = 1
var B = 2
var C = 3
func MyFunction(param1) do param1*param1 endfunc
if (D < A) then
  var A = -1
  func MyFunction(param1) do param1/2 endfunc
  MyFunction(A)
  if (A < B) then
    var A = B + 2
    D = D + A
  endif
  var B = -2
  C = A - B
  if (A > B) then
    var B = A + 2
    D = D + B
  endif
  var C = A + B
  D = D + C
endif
";

#[test]
fn code_blocks_determine_the_span_of_declarations() {
    // Page 1075 (Example 25.31): D ends at 2, the outer C was set to 1 inside, and the sum of
    // the outer A, B, C and the object D is 6. The page's closing call of MyFunction(A) reaches
    // the original function, whose value for A = 1 is 1.
    let mut form = Form::with(&[("D", Value::Number(9.0))]);
    let sum = run(
        &format!("{SPANS} A + B + C + D"),
        &mut form,
        Budget::CALCULATION,
    )
    .unwrap();
    assert_eq!(sum, Value::Number(6.0));
    assert_eq!(form.fields["D"], Value::Number(2.0));
    let mut form = Form::with(&[("D", Value::Number(9.0))]);
    let called = run(
        &format!("{SPANS} MyFunction(A)"),
        &mut form,
        Budget::CALCULATION,
    )
    .unwrap();
    assert_eq!(called, Value::Number(1.0));
}

#[test]
fn sum_adds_its_arguments_and_the_occurrences_of_an_object() {
    // Page 1090 (Example 25.44).
    assert!((number("Sum(1, 2, 3, 4)") - 10.0).abs() < 1e-9);
    let amounts = [
        ("Amount[0]", Value::Number(1.0)),
        ("Amount[1]", Value::Number(2.0)),
        ("Amount[2]", Value::Number(4.0)),
        ("Amount[3]", Value::Number(8.0)),
    ];
    let mut form = Form::with(&amounts);
    assert_eq!(
        run("Sum(Amount[*])", &mut form, Budget::CALCULATION).unwrap(),
        Value::Number(15.0)
    );
    assert_eq!(
        run("Sum(Amount[2], Amount[3])", &mut form, Budget::CALCULATION).unwrap(),
        Value::Number(12.0)
    );
}

#[test]
fn has_value_is_false_for_a_blank_and_true_for_zero() {
    // Page 1117 (Example 25.70).
    assert!((number(r#"HasValue(" ")"#)).abs() < 1e-9);
    assert!((number("HasValue(0)") - 1.0).abs() < 1e-9);
    let mut form = Form::with(&[("Item", Value::String("x".to_owned()))]);
    assert_eq!(
        run("HasValue(Item)", &mut form, Budget::CALCULATION).unwrap(),
        Value::Number(1.0)
    );
    assert_eq!(
        run("HasValue(Missing)", &mut form, Budget::CALCULATION).unwrap(),
        Value::Number(0.0)
    );
}

#[test]
fn concat_joins_and_treats_null_as_empty() {
    // Pages 1122 and 1135 (Examples 25.74 and 25.86).
    assert_eq!(text(r#"Concat("ABC", "CDE")"#), "ABCCDE");
    assert_eq!(text(r#"Concat("Hello ", null, "world.")"#), "Hello world.");
    let mut form = Form::with(&[("Item", Value::String("ABC".to_owned()))]);
    assert_eq!(
        run(
            r#"Concat("XX", Item, "-01")"#,
            &mut form,
            Budget::CALCULATION
        )
        .unwrap(),
        Value::String("XXABC-01".to_owned())
    );
}

#[test]
fn replace_substitutes_or_deletes() {
    // Page 1132 (Example 25.83).
    assert_eq!(
        text(r#"Replace("it's a dog's life", "dog", "cat")"#),
        "it's a cat's life"
    );
    assert_eq!(
        text(r#"Replace("it's a dog's life", "dog's ")"#),
        "it's a life"
    );
}

#[test]
fn an_assignment_sets_the_object_and_is_worth_its_value() {
    // Page 1066: the value of an assignment is the value assigned.
    let mut form = Form::with(&[("Total", Value::Null), ("A", Value::Number(2.0))]);
    let value = run("Total = A * 21", &mut form, Budget::CALCULATION).unwrap();
    assert_eq!(value, Value::Number(42.0));
    assert_eq!(form.fields["Total"], Value::Number(42.0));
}

#[test]
fn a_runaway_loop_meets_the_step_budget() {
    // Page 380 recommends a means of ending a loop that seems never to end.
    let budget = Budget {
        steps: 1000,
        ..Budget::CALCULATION
    };
    let outcome = run(
        "var i = 0 while (1) do i = i + 1 endwhile",
        &mut Form::default(),
        budget,
    );
    assert_eq!(outcome, Err(Stop::Exceeded("step")));
}

#[test]
fn a_string_grown_past_the_budget_stops() {
    let budget = Budget {
        string_bytes: 16,
        ..Budget::CALCULATION
    };
    let outcome = run(
        r#"Concat("0123456789", "0123456789")"#,
        &mut Form::default(),
        budget,
    );
    assert_eq!(outcome, Err(Stop::Exceeded("string length")));
}

#[test]
fn recursion_meets_the_call_budget() {
    let outcome = eval("func f(n) do f(n) endfunc f(1)");
    assert_eq!(outcome, Err(Stop::Exceeded("function call depth")));
}

#[test]
fn nesting_deeper_than_the_parser_allows_is_refused() {
    let source = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
    assert!(matches!(eval(&source), Err(Stop::Parse(_))));
}

#[test]
fn a_method_call_and_the_undocumented_functions_are_refused_by_name() {
    assert!(matches!(eval("Item.foo()"), Err(Stop::Unsupported(_))));
    assert!(matches!(eval("Sqrt(4)"), Err(Stop::Unsupported(_))));
}

#[test]
fn a_long_chain_of_operators_is_refused_and_never_overflows_the_stack() {
    // The parser bounds how deep binary operators stand in the tree it builds, because a chain is
    // a tree one level deeper per operator and the evaluator and the drop walk it recursively. The run is on
    // a thread with the smallest stack a worker is likely to have, where an unbounded walk would
    // abort the process.
    let outcome = std::thread::Builder::new()
        .stack_size(256 << 10)
        .spawn(|| {
            let chain = vec!["1"; 100_000].join(" + ");
            let mixed = vec!["1"; 100_000].join(" * 1 + ");
            (
                eval(&chain),
                eval(&mixed),
                eval(&vec!["1"; 150].join(" + ")),
            )
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(matches!(outcome.0, Err(Stop::Parse(_))), "{:?}", outcome.0);
    assert!(matches!(outcome.1, Err(Stop::Parse(_))), "{:?}", outcome.1);
    assert_eq!(
        outcome.2,
        Ok(Value::Number(150.0)),
        "a chain inside the bound still runs"
    );
}

/// `n` parenthesised levels, each the level inside it followed by the chains `ops` names, `k`
/// operators each: the shape whose depth is the sum of every level's chains.
fn nested(n: usize, ops: &[&str], k: usize) -> String {
    let mut script = "1".to_owned();
    for _ in 0..n {
        let mut level = script;
        for op in ops {
            level.push_str(&format!(" {op} 1").repeat(k));
        }
        script = format!("({level})");
    }
    script
}

#[test]
fn nested_chains_are_refused_by_the_depth_of_their_operators_and_never_overflow_the_stack() {
    // A chain whose first operand is a parenthesised chain is as deep as the two summed, though
    // neither chain is long: the bound is on the tree's depth (`MAX_OPERATOR_DEPTH`, ADR 2482),
    // so thirty levels of two hundred `+` and twenty levels of six chains of 199 are refused on a
    // small stack where an unbounded walk would abort the process.
    let outcome = std::thread::Builder::new()
        .stack_size(256 << 10)
        .spawn(|| {
            let six = ["*", "+", "<", "==", "&", "|"];
            (
                eval(&nested(30, &["+"], 200)),
                eval(&nested(20, &six, 199)),
                eval(&nested(25, &["+"], 8)),
            )
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(matches!(outcome.0, Err(Stop::Parse(_))), "{:?}", outcome.0);
    assert!(matches!(outcome.1, Err(Stop::Parse(_))), "{:?}", outcome.1);
    assert_eq!(
        outcome.2,
        Ok(Value::Number(201.0)),
        "twenty-five levels of eight, two hundred deep, still run"
    );
}
