//! The port of d3-force, checked against d3's own JavaScript.

use std::fs;

use d3::force::{self, Force, Link, Node};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/force.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

fn same_number(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

fn same_node(actual: &Node, expected: &Value) -> bool {
    same_number(actual.x, f64::from_bits(bits(&expected["x"])))
        && same_number(actual.y, f64::from_bits(bits(&expected["y"])))
        && same_number(actual.vx, f64::from_bits(bits(&expected["vx"])))
        && same_number(actual.vy, f64::from_bits(bits(&expected["vy"])))
}

#[test]
fn the_center_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let mut nodes = vec![
        Node::new(1.0, 2.0),
        Node::new(3.0, 4.0),
        Node::new(5.0, 6.0),
    ];
    for (i, node) in nodes.iter_mut().enumerate() {
        node.index = i;
    }
    let mut force = force::force_center(0.0, 0.0);
    force.initialize(&nodes);
    force.apply(1.0, &mut nodes);
    let expected = f["center"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
}

#[test]
fn the_x_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let mut nodes = vec![Node::new(10.0, 5.0), Node::new(2.0, 8.0)];
    for (i, node) in nodes.iter_mut().enumerate() {
        node.index = i;
    }
    let mut fx = force::force_x(4.0);
    fx.initialize(&nodes);
    fx.apply(0.5, &mut nodes);
    let expected = f["x"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
}

#[test]
fn the_y_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let mut nodes = vec![Node::new(10.0, 5.0), Node::new(2.0, 8.0)];
    for (i, node) in nodes.iter_mut().enumerate() {
        node.index = i;
    }
    let mut fy = force::force_y(3.0);
    fy.initialize(&nodes);
    fy.apply(0.5, &mut nodes);
    let expected = f["y"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
}

#[test]
fn the_many_body_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let mut nodes = vec![
        Node::new(0.0, 0.0),
        Node::new(1.0, 0.0),
        Node::new(0.0, 1.0),
    ];
    for (i, node) in nodes.iter_mut().enumerate() {
        node.index = i;
    }
    let mut fm = force::force_many_body();
    fm.initialize(&nodes);
    fm.apply(0.1, &mut nodes);
    let expected = f["manyBody"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
}

#[test]
fn the_collide_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let mut nodes = vec![Node::new(0.0, 0.0), Node::new(0.5, 0.0)];
    for (i, node) in nodes.iter_mut().enumerate() {
        node.index = i;
    }
    let mut fc = force::force_collide(1.0);
    fc.initialize(&nodes);
    fc.apply(1.0, &mut nodes);
    let expected = f["collide"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
}

#[test]
fn the_link_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let mut nodes = vec![Node::new(0.0, 0.0), Node::new(10.0, 0.0)];
    for (i, node) in nodes.iter_mut().enumerate() {
        node.index = i;
    }
    let links = vec![Link {
        source: 0,
        target: 1,
        index: 0,
    }];
    let mut fl = force::force_link(links);
    fl.initialize(&nodes);
    fl.apply(0.5, &mut nodes);
    let expected = f["link"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
}

#[test]
fn the_radial_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let mut nodes = vec![Node::new(1.0, 1.0), Node::new(2.0, 2.0)];
    for (i, node) in nodes.iter_mut().enumerate() {
        node.index = i;
    }
    let mut fr = force::force_radial(5.0, 0.0, 0.0);
    fr.initialize(&nodes);
    fr.apply(0.2, &mut nodes);
    let expected = f["radial"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
}

#[test]
fn the_simulation_is_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let nodes = vec![
        Node::new(0.0, 0.0),
        Node::new(10.0, 0.0),
        Node::new(0.0, 10.0),
    ];
    let mut sim = force::force_simulation(nodes);
    sim.set_force("center", Box::new(force::force_center(5.0, 5.0)));
    sim.set_force("x", Box::new(force::force_x(5.0)));
    let mut charge = force::force_many_body();
    charge.strength(-10.0);
    sim.set_force("charge", Box::new(charge));
    sim.tick(Some(5));
    let nodes = sim.get_nodes();
    let expected = f["simulation"].as_array().unwrap();
    assert_eq!(nodes.len(), expected.len());
    for (actual, exp) in nodes.iter().zip(expected.iter()) {
        assert!(same_node(actual, exp));
    }
    let alpha = sim.alpha();
    assert!(same_number(
        alpha,
        f64::from_bits(bits(&f["simulationAlpha"]))
    ));
}

#[test]
fn the_constant_and_jiggle_are_the_javascripts() {
    let f = fixture();
    force::reset_jiggle(1);
    let node = Node::new(0.0, 0.0);
    let nodes = vec![node.clone()];
    let c = force::constant(7.0);
    assert!(same_number(c(&node, 0, &nodes), 7.0));

    let expected = f["jiggle"].as_array().unwrap();
    for (i, exp) in expected.iter().enumerate() {
        let value = force::jiggle();
        assert!(
            same_number(value, f64::from_bits(bits(exp))),
            "jiggle {}",
            i
        );
    }
}
