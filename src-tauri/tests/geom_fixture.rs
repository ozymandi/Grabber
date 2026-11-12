//! Shared geometry fixture (design D5).
//!
//! `tests/fixtures/quads.json` is generated from `geom.rs` and read by both this test and
//! `ui/js/geom.test.mjs`, so the preview (JS) and the export (Rust) cannot drift apart.
//!
//! Regenerate after an intentional change to `geom.rs`:
//!   cargo test --test geom_fixture -- --ignored regenerate

use ripper_core::geom::{self, Aspect, Pt};
use serde_json::{json, Value};
use std::path::PathBuf;

const TOL: f64 = 1e-9;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/quads.json")
}

/// Input cases: name, corners as clicked, aspect, power-of-two.
fn inputs() -> Vec<(&'static str, [Pt; 4], Aspect, bool)> {
    let r11 = Aspect::Ratio { w: 1.0, h: 1.0 };
    let r43 = Aspect::Ratio { w: 4.0, h: 3.0 };
    let r21 = Aspect::Ratio { w: 2.0, h: 1.0 };
    let r31 = Aspect::Ratio { w: 3.0, h: 1.0 };
    let dy = (200.0_f64 * 200.0 - 50.0 * 50.0).sqrt();
    let spec = [[0.0, 0.0], [400.0, 0.0], [350.0, dy], [50.0, dy]];
    vec![
        ("skewed", [[120.5, 80.25], [910.0, 140.0], [870.0, 610.0], [60.0, 700.0]], Aspect::Edges, false),
        ("spec_edges", spec, Aspect::Edges, false),
        ("spec_1_1", spec, r11, false),
        ("spec_pot", spec, Aspect::Edges, true),
        ("custom_3_1", [[0.0, 0.0], [600.0, 0.0], [600.0, 90.0], [0.0, 90.0]], r31, false),
        ("ratio_4_3", [[10.0, 20.0], [530.0, 60.0], [500.0, 420.0], [40.0, 380.0]], r43, false),
        ("ratio_2_1_pot", [[10.0, 20.0], [530.0, 60.0], [500.0, 420.0], [40.0, 380.0]], r21, true),
        ("counter_clockwise", [[120.5, 80.25], [60.0, 700.0], [870.0, 610.0], [910.0, 140.0]], Aspect::Edges, false),
        ("parallelogram", [[10.0, 10.0], [110.0, 30.0], [130.0, 130.0], [30.0, 110.0]], Aspect::Edges, false),
        ("steep_floor", [[1700.0, 1100.0], [2140.0, 1100.0], [3600.0, 2100.0], [240.0, 2100.0]], Aspect::Edges, false),
        ("past_image_edge", [[-200.0, -50.0], [900.0, 10.0], [880.0, 700.0], [-150.0, 760.0]], Aspect::Edges, false),
        ("bow_tie", [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0], [100.0, 100.0]], Aspect::Edges, false),
        ("concave", [[0.0, 0.0], [100.0, 0.0], [30.0, 30.0], [0.0, 100.0]], Aspect::Edges, false),
        ("collinear", [[0.0, 0.0], [50.0, 0.0], [100.0, 0.0], [50.0, 80.0]], Aspect::Edges, false),
        ("tiny", [[0.0, 0.0], [1.5, 0.0], [1.5, 1.5], [0.0, 1.5]], Aspect::Edges, false),
    ]
}

fn mat(m: Option<geom::Mat3>) -> Value {
    match m {
        Some(m) => json!(m),
        None => Value::Null,
    }
}

fn compute(corners: &[Pt; 4], aspect: Aspect, pot: bool) -> Value {
    let cw = geom::clockwise(corners);
    let valid = match geom::validate(&cw) {
        Ok(()) => json!(true),
        Err(e) => json!(e),
    };
    let (w, h) = geom::output_size(&cw, aspect, pot);
    json!({
        "clockwise": cw,
        "valid": valid,
        "size": [w, h],
        "square_to_quad": mat(geom::square_to_quad(&cw)),
        "output_to_source": mat(geom::output_to_source(&cw, w, h)),
    })
}

fn build() -> Value {
    let cases: Vec<Value> = inputs()
        .into_iter()
        .map(|(name, corners, aspect, pot)| {
            json!({
                "name": name,
                "corners": corners,
                "aspect": aspect,
                "pot": pot,
                "expected": compute(&corners, aspect, pot),
            })
        })
        .collect();
    json!({
        "note": "Generated from src-tauri/src/geom.rs by tests/geom_fixture.rs; do not edit by hand.",
        "tolerance": TOL,
        "cases": cases,
    })
}

#[test]
#[ignore]
fn regenerate() {
    let path = fixture_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_string_pretty(&build()).unwrap() + "\n").unwrap();
}

fn assert_close(path: &str, a: &Value, b: &Value) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            assert!((x - y).abs() <= TOL * (1.0 + y.abs()), "{path}: {x} != {y}");
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{path}: length");
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                assert_close(&format!("{path}[{i}]"), p, q);
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            assert_eq!(x.len(), y.len(), "{path}: keys");
            for (k, p) in x {
                assert_close(&format!("{path}.{k}"), p, &y[k]);
            }
        }
        _ => assert_eq!(a, b, "{path}"),
    }
}

#[test]
fn fixture_matches_geom() {
    let text = std::fs::read_to_string(fixture_path())
        .expect("fixture missing: run `cargo test --test geom_fixture -- --ignored regenerate`");
    let stored: Value = serde_json::from_str(&text).unwrap();
    let cases = stored["cases"].as_array().unwrap();
    assert_eq!(cases.len(), inputs().len(), "case count");
    for case in cases {
        let corners: [Pt; 4] = serde_json::from_value(case["corners"].clone()).unwrap();
        let aspect: Aspect = serde_json::from_value(case["aspect"].clone()).unwrap();
        let pot = case["pot"].as_bool().unwrap();
        let name = case["name"].as_str().unwrap();
        assert_close(name, &compute(&corners, aspect, pot), &case["expected"]);
    }
}
