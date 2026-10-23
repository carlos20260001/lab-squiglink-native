//! The port of d3-chord, checked against d3's own JavaScript.

use std::fs;

use d3::chord::{chord, ribbon, ChordLayout, Chords, Group, Ribbon, Subgroup};
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
fn matrix() -> Vec<&'static [f64]> {
    static M0: [f64; 6] = [0.0, 5.0, 6.0, 4.0, 7.0, 4.0];
    static M1: [f64; 6] = [7.0, 0.0, 5.0, 6.0, 4.0, 7.0];
    static M2: [f64; 6] = [6.0, 7.0, 0.0, 5.0, 6.0, 4.0];
    static M3: [f64; 6] = [4.0, 6.0, 7.0, 0.0, 5.0, 6.0];
    static M4: [f64; 6] = [6.0, 4.0, 6.0, 7.0, 0.0, 5.0];
    static M5: [f64; 6] = [5.0, 6.0, 4.0, 6.0, 7.0, 0.0];
    vec![&M0, &M1, &M2, &M3, &M4, &M5]
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

fn check_subgroup(subgroup: &Subgroup, expected: &Json) {
    assert_eq!(subgroup.index, expected["index"].as_u64().unwrap() as usize);
    assert_eq!(
        subgroup.subindex,
        expected["subindex"].as_u64().unwrap() as usize
    );
    assert_eq!(subgroup.start_angle, number(&expected["startAngle"]));
    assert_eq!(subgroup.end_angle, number(&expected["endAngle"]));
    assert_eq!(subgroup.value, number(&expected["value"]));
}

fn check_chords(chords: &[d3::chord::Chord], expected: &Json) {
    let expected = expected.as_array().expect("the chords");
    assert_eq!(chords.len(), expected.len());
    for (chord, expected) in chords.iter().zip(expected) {
        check_subgroup(&chord.source, &expected["source"]);
        check_subgroup(&chord.target, &expected["target"]);
    }
}

fn check_section(chords: &Chords, expected: &Json) {
    check_groups(&chords.groups, expected.get("groups").expect("the groups"));
    check_chords(&chords.chords, expected.get("chords").expect("the chords"));
}

#[test]
fn the_chords_are_laid_out_the_javascripts_way() {
    let fixture = fixture();
    let chords = chord(&matrix());
    check_section(&chords, fixture.get("chord").expect("the chord"));
}

#[test]
fn the_pad_angle_is_the_javascripts_way() {
    let fixture = fixture();
    let chords = ChordLayout::new().pad_angle(0.05).apply(&matrix());
    check_section(&chords, fixture.get("chordPad").expect("the chord"));
}

#[test]
fn the_groups_are_sorted_the_javascripts_way() {
    let fixture = fixture();
    let chords = ChordLayout::new()
        .sort_groups(|a, b| b - a)
        .apply(&matrix());
    check_section(&chords, fixture.get("chordSort").expect("the chord"));
}

#[test]
fn the_subgroups_are_sorted_the_javascripts_way() {
    let fixture = fixture();
    let chords = ChordLayout::new()
        .sort_subgroups(|a, b| b - a)
        .apply(&matrix());
    check_section(&chords, fixture.get("chordSub").expect("the chord"));
}

#[test]
fn the_chords_are_sorted_the_javascripts_way() {
    let fixture = fixture();
    let chords = ChordLayout::new()
        .sort_chords(|a, b| b - a)
        .apply(&matrix());
    check_section(&chords, fixture.get("chordChord").expect("the chord"));
}

#[test]
fn the_setters_are_the_javascripts_getters() {
    let layout = ChordLayout::new()
        .pad_angle(-1.0)
        .sort_groups(|a, b| b - a)
        .sort_subgroups(|a, b| a - b)
        .sort_chords(|a, b| b - a);
    assert_eq!(layout.get_pad_angle(), 0.0);
    assert!(layout.get_sort_groups().is_some());
    assert!(layout.get_sort_subgroups().is_some());
    assert!(layout.get_sort_chords().is_some());
    assert!(layout.clear_sort_chords().get_sort_chords().is_none());
}

#[test]
fn the_ribbon_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("ribbon").expect("the ribbon");
    let chords = chord(&matrix());
    let chord = chords.chords[0].clone();
    let path = ribbon(&chord.source, &chord.target, 1.0);
    assert_eq!(path.text(), expected["path"].as_str().unwrap());
}

#[test]
fn the_ribbon_with_a_radius_is_drawn_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("ribbonRadius").expect("the ribbon");
    let chords = chord(&matrix());
    let chord = chords.chords[0].clone();
    let path = ribbon(&chord.source, &chord.target, 50.0);
    assert_eq!(path.text(), expected["path"].as_str().unwrap());
}

#[test]
fn the_ribbon_generator_is_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("ribbon").expect("the ribbon");
    let chords = chord(&matrix());
    let chord = chords.chords[0].clone();
    let generator: Ribbon<d3::chord::Chord> = Ribbon::default().radius(|_| 1.0);
    assert_eq!(generator.apply(&chord), expected["path"].as_str().unwrap());
}
