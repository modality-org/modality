use modality_lang::data::machine::run;
use modality_lang::data::{Machine, PathType, Value};

fn model() -> modality_lang::Model {
    let src = std::fs::read_to_string("tests/fixtures/leftpad/leftpad.modality").unwrap();
    modality_lang::parse_all_models_content_lalrpop(&src)
        .unwrap()
        .remove(0)
}

fn input(path: &str, raw: &str) -> (String, Value) {
    (path.to_string(), PathType::of(path).parse_value(raw).unwrap())
}

#[test]
fn leftpad_pads_foo_to_five() {
    let m = Machine::new(&model()).unwrap();
    let r = run(
        &m,
        &[
            input("/in/fill.text", "0"),
            input("/in/width.integer", "5"),
            input("/in/string.text", "foo"),
        ],
        100,
    );
    assert_eq!(r.path(&m), "q0 -> q1 -> q2 -> q2 -> q2 -> q3");
    assert_eq!(
        r.final_state.get("/out/result.text"),
        Some(&Value::Text("00foo".into()))
    );
}
