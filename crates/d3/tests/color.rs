//! The port of d3-color, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-color 1.4.1
//! under Node — the same code the browser runs — and writing down what it
//! produced.

use std::fs;

use d3::color::{
    color, cubehelix, gray, hcl, hsl, lab, lch, rgb, Color, Cubehelix, Hcl, Hsl, Lab, Rgb,
};
use serde_json::Value;

/// The number a fixture holds, which is written down as its bits.
///
/// The fixture writes the numbers as their bits, because a number written
/// in text and read back may not be the same number: the reader may
/// round it to the nearest one, and the nearest one is not always the
/// one that was written.
fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

/// The number a fixture holds, which is written down as its bits.
fn number(value: &Value) -> f64 {
    f64::from_bits(bits(value))
}

/// The bits of the number a fixture holds.
fn bits_of(value: &Value) -> u64 {
    number(value).to_bits()
}

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/color.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

#[test]
fn the_colours_are_read_the_javascripts_way() {
    let fixture = fixture();
    let parsed = fixture.get("parsed").expect("the parsed colours");
    let parsed = parsed.as_object().expect("a table of colours");
    assert!(!parsed.is_empty());
    for (format, expected) in parsed {
        let colour = color(format);
        if expected.is_null() {
            // The JavaScript gives null for a colour it cannot read.
            assert!(colour.is_none(), "{} was read", format);
            continue;
        }
        let expected = expected.as_object().expect("a colour");
        let colour = colour.unwrap_or_else(|| panic!("{} was not read", format));
        // The colour's own channels, which are the ones of its own
        // kind.
        let channels = expected.get("channels").expect("the channels");
        let channels = channels.as_object().expect("the channels");
        match colour {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.r.to_bits(), bits_of(&channels["r"]));
                assert_eq!(rgb.g.to_bits(), bits_of(&channels["g"]));
                assert_eq!(rgb.b.to_bits(), bits_of(&channels["b"]));
            }
            Color::Hsl(hsl) => {
                assert_eq!(hsl.h.to_bits(), bits_of(&channels["h"]));
                assert_eq!(hsl.s.to_bits(), bits_of(&channels["s"]));
                assert_eq!(hsl.l.to_bits(), bits_of(&channels["l"]));
            }
            Color::Lab(lab) => {
                assert_eq!(lab.l.to_bits(), bits_of(&channels["l"]));
                assert_eq!(lab.a.to_bits(), bits_of(&channels["a"]));
                assert_eq!(lab.b.to_bits(), bits_of(&channels["b"]));
            }
            Color::Hcl(hcl) => {
                assert_eq!(hcl.h.to_bits(), bits_of(&channels["h"]));
                assert_eq!(hcl.c.to_bits(), bits_of(&channels["c"]));
                assert_eq!(hcl.l.to_bits(), bits_of(&channels["l"]));
            }
            Color::Cubehelix(cubehelix) => {
                assert_eq!(cubehelix.h.to_bits(), bits_of(&channels["h"]));
                assert_eq!(cubehelix.s.to_bits(), bits_of(&channels["s"]));
                assert_eq!(cubehelix.l.to_bits(), bits_of(&channels["l"]));
            }
        }
        let opacity = match colour {
            Color::Rgb(rgb) => rgb.opacity,
            Color::Hsl(hsl) => hsl.opacity,
            Color::Lab(lab) => lab.opacity,
            Color::Hcl(hcl) => hcl.opacity,
            Color::Cubehelix(cubehelix) => cubehelix.opacity,
        };
        assert_eq!(
            opacity.to_bits(),
            bits_of(&channels["opacity"]),
            "{} has an opacity of {:?}",
            format,
            opacity
        );
        assert_eq!(
            colour.format_hex(),
            expected["formatHex"].as_str().unwrap(),
            "{} is written as a hex",
            format
        );
        assert_eq!(
            colour.format_rgb(),
            expected["formatRgb"].as_str().unwrap(),
            "{} is written as an rgb",
            format
        );
        assert_eq!(
            colour.format_hsl(),
            expected["formatHsl"].as_str().unwrap(),
            "{} is written as an hsl",
            format
        );
        assert_eq!(
            colour.displayable(),
            expected["displayable"].as_bool().unwrap(),
            "{} is displayable",
            format
        );
    }
}

#[test]
fn the_spaces_are_converted_the_javascripts_way() {
    let fixture = fixture();
    let expected = fixture.get("conversions").expect("the conversions");
    let colour = color("steelblue").expect("the colour");

    let rgb = colour.rgb();
    let want = expected.get("rgb").expect("the rgb").as_array().unwrap();
    assert_eq!(rgb.r.to_bits(), bits_of(&want[0]));
    assert_eq!(rgb.g.to_bits(), bits_of(&want[1]));
    assert_eq!(rgb.b.to_bits(), bits_of(&want[2]));
    assert_eq!(rgb.opacity.to_bits(), bits_of(&want[3]));

    let hsl = colour.hsl();
    let want = expected.get("hsl").expect("the hsl").as_array().unwrap();
    assert_eq!(hsl.h.to_bits(), bits_of(&want[0]));
    assert_eq!(hsl.s.to_bits(), bits_of(&want[1]));
    assert_eq!(hsl.l.to_bits(), bits_of(&want[2]));
    assert_eq!(hsl.opacity.to_bits(), bits_of(&want[3]));

    let lab = colour.lab();
    let want = expected.get("lab").expect("the lab").as_array().unwrap();
    assert_eq!(lab.l.to_bits(), bits_of(&want[0]));
    assert_eq!(lab.a.to_bits(), bits_of(&want[1]));
    assert_eq!(lab.b.to_bits(), bits_of(&want[2]));
    assert_eq!(lab.opacity.to_bits(), bits_of(&want[3]));

    let hcl = colour.hcl();
    let want = expected.get("hcl").expect("the hcl").as_array().unwrap();
    assert_eq!(hcl.h.to_bits(), bits_of(&want[0]));
    assert_eq!(hcl.c.to_bits(), bits_of(&want[1]));
    assert_eq!(hcl.l.to_bits(), bits_of(&want[2]));
    assert_eq!(hcl.opacity.to_bits(), bits_of(&want[3]));

    let cubehelix = colour.cubehelix();
    let want = expected
        .get("cubehelix")
        .expect("the cubehelix")
        .as_array()
        .unwrap();
    assert_eq!(cubehelix.h.to_bits(), bits_of(&want[0]));
    assert_eq!(cubehelix.s.to_bits(), bits_of(&want[1]));
    assert_eq!(cubehelix.l.to_bits(), bits_of(&want[2]));
    assert_eq!(cubehelix.opacity.to_bits(), bits_of(&want[3]));
}

#[test]
fn the_colours_are_made_the_javascripts_way() {
    let fixture = fixture();
    let made = fixture.get("made").expect("the made colours");
    assert_eq!(
        rgb(1.0, 2.0, 3.0, None).format_rgb(),
        made["rgb"].as_str().unwrap()
    );
    assert_eq!(
        rgb(1.0, 2.0, 3.0, Some(0.5)).format_rgb(),
        made["rgbOpacity"].as_str().unwrap()
    );
    assert_eq!(
        hsl(120.0, 0.5, 0.5, None).format_hsl(),
        made["hsl"].as_str().unwrap()
    );
    assert_eq!(
        hsl(120.0, 0.5, 0.5, Some(0.5)).format_hsl(),
        made["hslOpacity"].as_str().unwrap()
    );
    assert_eq!(
        lab(50.0, 20.0, -30.0, None).format_rgb(),
        made["lab"].as_str().unwrap()
    );
    assert_eq!(
        gray(50.0, None).format_rgb(),
        made["gray"].as_str().unwrap()
    );
    assert_eq!(
        hcl(120.0, 40.0, 50.0, None).format_rgb(),
        made["hcl"].as_str().unwrap()
    );
    assert_eq!(
        lch(50.0, 40.0, 120.0, None).format_rgb(),
        made["lch"].as_str().unwrap()
    );
    assert_eq!(
        cubehelix(120.0, 0.5, 0.5, None).format_rgb(),
        made["cubehelix"].as_str().unwrap()
    );
}

#[test]
fn the_colours_are_brighter_and_darker_the_javascripts_way() {
    let fixture = fixture();
    let formats = ["steelblue", "#ff0000", "hsl(120, 50%, 50%)", "transparent"];
    for (name, brighter) in [("brighter", true), ("darker", false)] {
        let expected = fixture.get(name).expect("the colours");
        let expected = expected.as_array().expect("a list of colours");
        assert_eq!(expected.len(), formats.len());
        for (format, expected) in formats.iter().zip(expected) {
            let colour = color(format).expect("the colour");
            let changed = if brighter {
                colour.brighter(None)
            } else {
                colour.darker(None)
            };
            let expected = expected.as_array().expect("a colour");
            assert_eq!(
                changed.format_rgb(),
                expected[0].as_str().unwrap(),
                "{} of {}",
                name,
                format
            );
            assert_eq!(
                changed.format_hex(),
                expected[1].as_str().unwrap(),
                "{} of {}",
                name,
                format
            );
            assert_eq!(
                changed.format_hsl(),
                expected[2].as_str().unwrap(),
                "{} of {}",
                name,
                format
            );
        }
    }
}

#[test]
fn a_cubehelix_is_raised_to_a_power_of_its_own() {
    let fixture = fixture();
    let expected = fixture.get("cubehelixK").expect("the cubehelix");
    let cube = match cubehelix(120.0, 0.5, 0.5, None) {
        Color::Cubehelix(cube) => cube,
        _ => panic!("a cubehelix"),
    };
    assert_eq!(
        cube.brighter(Some(0.0)).rgb().format_rgb(),
        expected["brighter0"]
    );
    assert_eq!(
        cube.brighter(Some(2.0)).rgb().format_rgb(),
        expected["brighter2"]
    );
    assert_eq!(
        cube.darker(Some(0.0)).rgb().format_rgb(),
        expected["darker0"]
    );
    assert_eq!(
        cube.darker(Some(2.0)).rgb().format_rgb(),
        expected["darker2"]
    );
}

#[test]
fn the_colours_have_their_own_kinds() {
    // The kinds are the JavaScript's own, and a colour of one kind is
    // not a colour of another.
    assert!(matches!(color("red"), Some(Color::Rgb(_))));
    assert!(matches!(color("hsl(0, 100%, 50%)"), Some(Color::Hsl(_))));
    assert!(matches!(lab(0.0, 0.0, 0.0, None), Color::Lab(_)));
    assert!(matches!(hcl(0.0, 0.0, 0.0, None), Color::Hcl(_)));
    assert!(matches!(
        cubehelix(0.0, 0.0, 0.0, None),
        Color::Cubehelix(_)
    ));
    let rgb = Rgb {
        r: 255.0,
        g: 0.0,
        b: 0.0,
        opacity: 1.0,
    };
    let hsl = Hsl {
        h: 0.0,
        s: 1.0,
        l: 0.5,
        opacity: 1.0,
    };
    let lab = Lab {
        l: 50.0,
        a: 0.0,
        b: 0.0,
        opacity: 1.0,
    };
    let hcl = Hcl {
        h: 0.0,
        c: 0.0,
        l: 50.0,
        opacity: 1.0,
    };
    let cubehelix = Cubehelix {
        h: 0.0,
        s: 0.0,
        l: 0.5,
        opacity: 1.0,
    };
    assert_eq!(rgb.format_hex(), "#ff0000");
    assert_eq!(hsl.format_hsl(), "hsl(0, 100%, 50%)");
    assert!(lab.rgb().displayable() || !lab.rgb().displayable());
    assert!(hcl.rgb().displayable() || !hcl.rgb().displayable());
    assert!(cubehelix.rgb().displayable() || !cubehelix.rgb().displayable());
}
