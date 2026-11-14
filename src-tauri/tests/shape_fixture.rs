//! Shared shape fixture (design D2 of add-quad-shapes).
//!
//! `tests/fixtures/shapes.json` is generated from `shape.rs` and read by both this test and
//! `ui/js/shape.test.mjs`, so the preview and canvas (JS) and the export (Rust) agree.
//!
//! Regenerate after an intentional change to `shape.rs`:
//!   cargo test --test shape_fixture -- --ignored regenerate

use ripper_core::shape::{self, Node, Shape};
use serde_json::{json, Value};
use std::path::PathBuf;

const TOL: f64 = 1e-9;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shapes.json")
}

fn mixed_path() -> Shape {
    Shape::Path {
        nodes: vec![
            Node::corner([0.1, 0.1]),
            Node { p: [0.9, 0.2], hin: Some([0.6, 0.0]), hout: Some([1.0, 0.5]), smooth: false },
            Node { p: [0.7, 0.9], hin: Some([0.95, 0.85]), hout: Some([0.45, 0.95]), smooth: true },
            Node { p: [0.2, 0.6], hin: None, hout: Some([0.05, 0.4]), smooth: false },
        ],
    }
}

/// Input cases: name, shape, output width and height.
fn inputs() -> Vec<(String, Shape, f64, f64)> {
    let mut v = vec![
        ("rect".to_string(), Shape::Rect, 300.0, 200.0),
        ("circle_64".to_string(), Shape::Circle, 64.0, 64.0),
        ("ellipse_300x120".to_string(), Shape::Circle, 300.0, 120.0),
        ("circle_path_256".to_string(), Shape::Path { nodes: shape::to_path(&Shape::Circle) }, 256.0, 256.0),
        ("mixed_path_400x300".to_string(), mixed_path(), 400.0, 300.0),
    ];
    for n in 3..=12 {
        v.push((format!("ngon_{n}"), Shape::Ngon { n }, 512.0, 384.0));
    }
    v
}

fn build() -> Value {
    let cases: Vec<Value> = inputs()
        .into_iter()
        .map(|(name, s, w, h)| {
            json!({
                "name": name,
                "shape": s,
                "w": w,
                "h": h,
                "expected": {
                    "outline": shape::outline(&s, w, h),
                    "to_path": shape::to_path(&s),
                    "empty": shape::is_empty(&s, w as u32, h as u32),
                }
            })
        })
        .collect();
    let path = mixed_path();
    let Shape::Path { nodes } = &path else { unreachable!() };
    let splits: Vec<Value> = (0..nodes.len())
        .map(|i| json!({ "nodes": nodes, "seg": i, "t": 0.37, "expected": shape::split(nodes, i, 0.37) }))
        .collect();
    json!({ "tolerance": TOL, "cases": cases, "splits": splits })
}

fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() <= TOL * (1.0 + y.abs())
        }
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w))),
        _ => a == b,
    }
}

#[test]
fn fixture_matches_shape_rs() {
    let want: Value = serde_json::from_str(&std::fs::read_to_string(fixture_path()).expect("fixture")).unwrap();
    assert!(close(&build(), &want), "shapes.json is out of date; regenerate it");
}

#[test]
#[ignore]
fn regenerate() {
    std::fs::write(fixture_path(), serde_json::to_string_pretty(&build()).unwrap()).unwrap();
}
