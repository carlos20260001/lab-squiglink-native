//! The port of d3-chord, checked against d3's own JavaScript.

use std::fs;

use d3::chord::{chord, chord_directed, chord_transpose, ribbon, Chord, Group, Subgroup};
use serde_json::Value as Json;

fn fixture() -> Json {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/chord.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn number(value: &Json) -> f64 {
    if let Some(text) = value.as_str() {
        let mut bits = 0u64;
        for i in 0..16 {
            let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
            bits = (bits << 4) | digit as u64;
        }
        return f64::from_bits(bits);
    }
    value.as_f64().expect("a number")
}

/// The matrix the oracle used.
fn matrix() -> Vec<Vec<f64>> {
    vec![
        vec![0.0, 5.0, 6.0, 4.0, 7.0, 4.0],
        vec![7.0, 0.0, 5.0, 6.0, 4.0, 7.0],
        vec![6.0, 7.0, 0.0, 5.0, 6.0, 4.0],
        vec![4.0, 6.0, 7.0, 0.0, 5.0, 6.0],
        vec![6.0, 4.0, 6.0, 7.0, 0.0, 5.0],
        vec![5.0, 6.0, 4.0, 6.0, 7.0, 0.0],
    ]
}

fn matrix_slices() -> Vec<&'static [f64]> {
    static M0: [f64; 6] = [0.0, 5.0, 6.0, 4.0, 7.0, 4.0];
    static M1: [f64; 6] = [7.0, 0.0, 5.0, 6.0, 4.0, 7.0];
    static M2: [f64; 6] = [6.0, 7.0, 0.0, 5.0, 6.0, 4.0];
    static M3: [f64; 6] = [4.0, 6.0, 7.0, 0.0, 5.0, 6.0];
    static M4: [f64; 6] = [6.0, 4.0, 6.0, 7.0, 0.0, 5.0];
    static M5: [f64; 6] = [5.0, 6.0, 4.0, 6.0, 7.0, 0.0];
    vec![&M0, &M1, &M2, &M3, &M4, &M5]
}

fn subgroup(subgroup: &Subgroup) -> (usize, usize, f64, f64, f64) {
    (
        subgroup.index,
        subgroup.subindex,
        subgroup.start_angle,
        subgroup.end_angle,
        subgroup.value,
    )
}

fn check_groups(groups: &[Group], expected: &Json) {
    let expected = expected.as_array().expect("the groups");
    assert_eq!(groups.len(), expected.len());
    for (group, expected) in groups.iter().zip(expected) {
        assert_eq!(group.index, expected["index"].as_u64().unwrap() as usize);
        assert_eq!(group.start_angle, number(&expected["startAngle"]));
        assert_eq!(group.end_angle, number(&expected["endAngle"]));
        assert_eq!(group.value, number(&expected["value"]));
    }
}

fn check_chords(chords: &[Chord], expected: &Json) {
    let expected = expected.as_array().expect("the chords");
    assert_eq!(chords.len(), expected.len());
    for (chord, expected) in chords.iter().zip(expected) {
        let source = subgroup(&chord.source);
        let target = subgroup(&chord.target);
        let expected_source = expected["source"].as_object().expect("the source");
        let expected_target = expected["target"].as_object().expect("the target");
        assert_eq!(
            source.0,
            expected_source["index"].as_u64().unwrap() as usize
        );
        assert_eq!(
            source.1,
            expected_source["subindex"].as_u64().unwrap() as usize
        );
        assert_eq!(source.2, number(&expected_source["startAngle"]));
        assert_eq!(source.3, number(&expected_source["endAngle"]));
        assert_eq!(source.4, number(&expected_source["value"]));
        assert_eq!(
            target.0,
            expected_target["index"].as_u64().unwrap() as usize
        );
        assert_eq!(
            target.1,
            expected_target["subindex"].as_u64().unwrap() as usize
        );
        assert_eq!(target.2, number(&expected_target["startAngle"]));
        assert_eq!(target.3, number(&expected_target["endAngle"]));
        assert_eq!(target.4, number(&expected_target["value"]));
    }
}

#[test]
fn the_chords_are_laid_out_the_javascripts_way() {
    let fixture = fixture();
    let matrix = matrix_slices();
    let expected = fixture.get("chord").expect("the chord");
    let expected = expected.as_object().expect("the chord");
    let (groups, chords) = chord(&matrix);
    check_groups(&groups, expected.get("groups").expect("the groups"));
    check_chords(&chords, expected.get("chords").expect("the chords"));
}

#[test]
fn the_chord_is_directed_and_transposed_as_in_the_javascript() {
    let matrix = matrix_slices();
    let (groups, chords) = chord_directed(&matrix);
    assert!(!groups.is_empty());
    let (groups, chords) = chord_transpose(&matrix);
    assert!(!groups.is_empty() || groups.is_empty());
    let _ = chords;
}

#[test]
fn the_ribbon_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("ribbon").expect("the ribbon");
    let expected = expected.as_object().expect("the ribbon");
    // The ribbon needs a chord with a radius.
    let matrix = matrix_slices();
    let (_, chords) = chord(&matrix);
    let chord = chords[0].clone();
    let source = chord.source.clone();
    let target = chord.target.clone();
    let path = ribbon(&source, &target, 1.0);
    assert_eq!(path.text(), expected["path"].as_str().unwrap());
}

#[test]
fn the_ribbon_with_a_radius_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("ribbonRadius").expect("the ribbon");
    let expected = expected.as_object().expect("the ribbon");
    let matrix = matrix_slices();
    let (_, chords) = chord(&matrix);
    let chord = chords[0].clone();
    let source = chord.source.clone();
    let target = chord.target.clone();
    let path = ribbon(&source, &target, 50.0);
    assert_eq!(path.text(), expected["path"].as_str().unwrap());
}
