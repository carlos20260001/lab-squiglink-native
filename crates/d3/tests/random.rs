//! The port of d3-random, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-random under Node —
//! the same code the browser runs — and writing down what it produced. The
//! numbers are written down as their bits, for a number written in text and
//! read back may not be the same number.

use std::fs;

use d3::random;
use serde_json::Value;

/// The fixture, which the oracle wrote.
fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/random.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The number a fixture holds, which is written down as its bits.
fn number(value: &Value) -> f64 {
    let text = value.as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for index in 0..16 {
        let digit = u8::from_str_radix(&text[index..index + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    f64::from_bits(bits)
}

/// The numbers of a fixture's list.
fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("a list of numbers")
        .iter()
        .map(number)
        .collect()
}

/// Whether two numbers are the same number.
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

/// The seed of a case, which the oracle wrote down with the numbers.
fn seed(case: &Value) -> u32 {
    case["seed"].as_u64().expect("a seed") as u32
}

/// A source of the oracle's own, so that the numbers the JavaScript gave
/// are the numbers the port gives.
///
/// It is the linear congruential generator the oracle uses, and the seed is
/// the one the oracle wrote down.
fn lcg(case: &Value) -> random::Source {
    let mut state = seed(case);
    Box::new(move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(state) / 4_294_967_296.0
    })
}

/// A source that gives the numbers the oracle gave it, one after another,
/// and the last of them for ever after that.
fn fixed(case: &Value) -> random::Source {
    let values = numbers(&case["fixed"]);
    let mut index = 0;
    Box::new(move || {
        let value = values[index.min(values.len() - 1)];
        index += 1;
        value
    })
}

/// The numbers a generator gives, against the numbers the JavaScript gave.
macro_rules! check {
    ($generator:expr, $case:expr, $what:expr) => {{
        let case = &$case;
        let expected = numbers(&case["values"]);
        let mut generator = $generator;
        let mut got = Vec::with_capacity(expected.len());
        for _ in 0..expected.len() {
            got.push(generator.next());
        }
        assert_eq!(got.len(), expected.len(), "{}", $what);
        for (got, expected) in got.iter().zip(expected.iter()) {
            assert!(
                same(*got, *expected),
                "{}: the port gave {:016x}, the JavaScript gave {:016x}",
                $what,
                got.to_bits(),
                expected.to_bits()
            );
        }
    }};
}

#[test]
fn the_uniform_generator_gives_what_the_javascript_gives() {
    let f = fixture();

    // The JavaScript's `randomUniform` reads how many arguments it was
    // given, and the port's `random_uniform` follows the rule that a `min`
    // with no `max` is the JavaScript's one argument.
    check!(
        random::random_uniform_source(lcg(&f["uniform"]["noArgs"]), None, None),
        f["uniform"]["noArgs"],
        "uniform, no arguments"
    );
    check!(
        random::random_uniform_source(lcg(&f["uniform"]["oneArg"]), Some(5.0), None),
        f["uniform"]["oneArg"],
        "uniform, one argument"
    );
    check!(
        random::random_uniform_source(lcg(&f["uniform"]["twoArgs"]), Some(2.0), Some(5.0)),
        f["uniform"]["twoArgs"],
        "uniform, two arguments"
    );
    check!(
        random::random_uniform_source(lcg(&f["uniform"]["negative"]), Some(-3.0), Some(-1.0)),
        f["uniform"]["negative"],
        "uniform, negative bounds"
    );
    check!(
        random::random_uniform_source(lcg(&f["uniform"]["zeroWidth"]), Some(2.0), Some(2.0)),
        f["uniform"]["zeroWidth"],
        "uniform, no width"
    );
    check!(
        random::random_uniform_source(lcg(&f["uniform"]["maxBelowMin"]), Some(5.0), Some(2.0)),
        f["uniform"]["maxBelowMin"],
        "uniform, the upper bound below the lower one"
    );

    // The JavaScript's own spellings, which the port keeps in
    // `random_uniform_args`.
    check!(
        random::random_uniform_source_args(lcg(&f["uniform"]["nullArg"]), &[None]),
        f["uniform"]["nullArg"],
        "uniform, a null argument"
    );
    check!(
        random::random_uniform_source_args(lcg(&f["uniform"]["undefinedArg"]), &[None]),
        f["uniform"]["undefinedArg"],
        "uniform, an argument that was not there"
    );
    check!(
        random::random_uniform_source_args(lcg(&f["uniform"]["nullMin"]), &[None, Some(5.0)]),
        f["uniform"]["nullMin"],
        "uniform, a null lower bound"
    );
    check!(
        random::random_uniform_source_args(lcg(&f["uniform"]["undefinedMax"]), &[Some(2.0), None]),
        f["uniform"]["undefinedMax"],
        "uniform, an upper bound that was not there"
    );
    check!(
        random::random_uniform_source_args(
            lcg(&f["uniform"]["extraArgs"]),
            &[Some(1.0), Some(2.0), Some(3.0)]
        ),
        f["uniform"]["extraArgs"],
        "uniform, an argument too many"
    );
}

#[test]
fn the_normal_generator_gives_what_the_javascript_gives() {
    let f = fixture();

    check!(
        random::random_normal_source(lcg(&f["normal"]["noArgs"]), None, None),
        f["normal"]["noArgs"],
        "normal, no arguments"
    );
    check!(
        random::random_normal_source(lcg(&f["normal"]["mu"]), Some(5.0), None),
        f["normal"]["mu"],
        "normal, the middle"
    );
    check!(
        random::random_normal_source(lcg(&f["normal"]["sigma"]), None, Some(2.0)),
        f["normal"]["sigma"],
        "normal, the spread"
    );
    check!(
        random::random_normal_source(lcg(&f["normal"]["both"]), Some(3.0), Some(2.0)),
        f["normal"]["both"],
        "normal, both arguments"
    );
    check!(
        random::random_normal_source(lcg(&f["normal"]["negativeMu"]), Some(-3.0), Some(0.5)),
        f["normal"]["negativeMu"],
        "normal, a negative middle"
    );
    check!(
        random::random_normal_source(lcg(&f["normal"]["nulls"]), None, None),
        f["normal"]["nulls"],
        "normal, two null arguments"
    );

    // The JavaScript throws a pair of numbers away when the circle they are
    // in is not the unit one, and it keeps the first of the pair for the
    // next number.
    check!(
        random::random_normal_source(fixed(&f["normal"]["fixedSource"]), None, None),
        f["normal"]["fixedSource"],
        "normal, a source of the oracle's own"
    );
}

#[test]
fn the_log_normal_generator_gives_what_the_javascript_gives() {
    let f = fixture();

    check!(
        random::random_log_normal_source(lcg(&f["logNormal"]["noArgs"]), None, None),
        f["logNormal"]["noArgs"],
        "log-normal, no arguments"
    );
    check!(
        random::random_log_normal_source(lcg(&f["logNormal"]["mu"]), Some(5.0), None),
        f["logNormal"]["mu"],
        "log-normal, the middle"
    );
    check!(
        random::random_log_normal_source(lcg(&f["logNormal"]["sigma"]), None, Some(2.0)),
        f["logNormal"]["sigma"],
        "log-normal, the spread"
    );
    check!(
        random::random_log_normal_source(lcg(&f["logNormal"]["both"]), Some(3.0), Some(2.0)),
        f["logNormal"]["both"],
        "log-normal, both arguments"
    );
    check!(
        random::random_log_normal_source(lcg(&f["logNormal"]["negativeMu"]), Some(-3.0), Some(0.5)),
        f["logNormal"]["negativeMu"],
        "log-normal, a negative middle"
    );
}

#[test]
fn the_exponential_generator_gives_what_the_javascript_gives() {
    let f = fixture();

    check!(
        random::random_exponential_source(lcg(&f["exponential"]["one"]), Some(1.0)),
        f["exponential"]["one"],
        "exponential, one"
    );
    check!(
        random::random_exponential_source(lcg(&f["exponential"]["two"]), Some(2.0)),
        f["exponential"]["two"],
        "exponential, two"
    );
    check!(
        random::random_exponential_source(lcg(&f["exponential"]["half"]), Some(0.5)),
        f["exponential"]["half"],
        "exponential, a half"
    );
    check!(
        random::random_exponential_source(lcg(&f["exponential"]["zero"]), Some(0.0)),
        f["exponential"]["zero"],
        "exponential, a zero rate"
    );
    check!(
        random::random_exponential_source(lcg(&f["exponential"]["negative"]), Some(-1.0)),
        f["exponential"]["negative"],
        "exponential, a negative rate"
    );

    // The JavaScript's `null` is a division by zero, which the port's
    // caller writes as a zero.
    check!(
        random::random_exponential_source(lcg(&f["exponential"]["null"]), Some(0.0)),
        f["exponential"]["null"],
        "exponential, a null rate"
    );

    // The JavaScript's rate that was not there at all is a division by
    // nothing.
    check!(
        random::random_exponential_source(lcg(&f["exponential"]["undefined"]), None),
        f["exponential"]["undefined"],
        "exponential, a rate that was not there"
    );

    // A source that gives one, which is the end of the range the logarithm
    // is taken of, and a source that gives a number that is not a number.
    check!(
        random::random_exponential_source(fixed(&f["exponential"]["fixedSource"]), Some(1.0)),
        f["exponential"]["fixedSource"],
        "exponential, a source of the oracle's own"
    );
}

#[test]
fn the_bates_generator_gives_what_the_javascript_gives() {
    let f = fixture();

    for n in ["1", "2", "3", "5"] {
        let count = n.parse::<f64>().expect("a number");
        check!(
            random::random_bates_source(lcg(&f["bates"][n]), Some(count)),
            f["bates"][n],
            "bates, the number of the sum"
        );
    }

    check!(
        random::random_bates_source(lcg(&f["bates"]["zero"]), Some(0.0)),
        f["bates"]["zero"],
        "bates, no numbers in the sum"
    );
    check!(
        random::random_bates_source(lcg(&f["bates"]["negative"]), Some(-1.0)),
        f["bates"]["negative"],
        "bates, a negative number of numbers"
    );
    check!(
        random::random_bates_source(lcg(&f["bates"]["fraction"]), Some(2.5)),
        f["bates"]["fraction"],
        "bates, a number that is not whole"
    );

    // The JavaScript's `null` is a division by zero, which the port's
    // caller writes as a zero.
    check!(
        random::random_bates_source(lcg(&f["bates"]["null"]), Some(0.0)),
        f["bates"]["null"],
        "bates, a null number of numbers"
    );

    // The JavaScript's number of numbers that was not there at all is a
    // division by nothing.
    check!(
        random::random_bates_source(lcg(&f["bates"]["undefined"]), None),
        f["bates"]["undefined"],
        "bates, a number of numbers that was not there"
    );
}

#[test]
fn the_irwin_hall_generator_gives_what_the_javascript_gives() {
    let f = fixture();

    for n in ["1", "2", "3", "5"] {
        let count = n.parse::<f64>().expect("a number");
        check!(
            random::random_irwin_hall_source(lcg(&f["irwinHall"][n]), Some(count)),
            f["irwinHall"][n],
            "irwin-hall, the number of the sum"
        );
    }

    check!(
        random::random_irwin_hall_source(lcg(&f["irwinHall"]["zero"]), Some(0.0)),
        f["irwinHall"]["zero"],
        "irwin-hall, no numbers in the sum"
    );
    check!(
        random::random_irwin_hall_source(lcg(&f["irwinHall"]["negative"]), Some(-1.0)),
        f["irwinHall"]["negative"],
        "irwin-hall, a negative number of numbers"
    );
    check!(
        random::random_irwin_hall_source(lcg(&f["irwinHall"]["fraction"]), Some(2.5)),
        f["irwinHall"]["fraction"],
        "irwin-hall, a number that is not whole"
    );
    check!(
        random::random_irwin_hall_source(lcg(&f["irwinHall"]["null"]), Some(0.0)),
        f["irwinHall"]["null"],
        "irwin-hall, a null number of numbers"
    );
    check!(
        random::random_irwin_hall_source(lcg(&f["irwinHall"]["undefined"]), None),
        f["irwinHall"]["undefined"],
        "irwin-hall, a number of numbers that was not there"
    );
}

#[test]
fn the_default_source_gives_numbers_in_the_range() {
    let f = fixture();

    // The JavaScript's default source is `Math.random`, which the
    // specification does not give beyond that it is a number in [0, 1), and
    // the port's own source is the same kind of thing.
    let count = f["defaultSource"]["count"].as_u64().expect("a count") as usize;
    let expected = f["defaultSource"]["inRange"].as_u64().expect("a count") as usize;

    let mut source = random::default_source();
    let mut in_range = 0;
    for _ in 0..count {
        let value = source();
        if (0.0..1.0).contains(&value) {
            in_range += 1;
        }
    }
    assert_eq!(in_range, expected);
}

#[test]
fn the_generators_without_a_source_of_their_own_give_numbers_of_their_kind() {
    // The JavaScript's default source is `Math.random`, whose stream is the
    // engine's own and cannot be written down, so these are checked for
    // what they are: the numbers of the right kind, from the port's own
    // stream.

    let mut uniform = random::random_uniform(None, None);
    let mut one = random::random_uniform(Some(5.0), None);
    let mut two = random::random_uniform(Some(2.0), Some(5.0));
    let mut args = random::random_uniform_args(&[Some(1.0), Some(3.0)]);
    for _ in 0..100 {
        let value = uniform.next();
        assert!((0.0..1.0).contains(&value), "uniform: {}", value);
        let value = one.next();
        assert!(
            (0.0..5.0).contains(&value),
            "uniform, one argument: {}",
            value
        );
        let value = two.next();
        assert!(
            (2.0..5.0).contains(&value),
            "uniform, two arguments: {}",
            value
        );
        let value = args.next();
        assert!(
            (1.0..3.0).contains(&value),
            "uniform, the arguments: {}",
            value
        );
    }

    // The normal numbers are about their middle, and the spread is what
    // they are wide.
    let mut normal = random::random_normal(Some(10.0), Some(2.0));
    let mut sum = 0.0;
    for _ in 0..1000 {
        sum += normal.next();
    }
    let mean = sum / 1000.0;
    assert!((mean - 10.0).abs() < 0.5, "normal: the mean is {}", mean);

    // The log-normal numbers are the exponential of the normal ones, and
    // so they are all above zero.
    let mut log_normal = random::random_log_normal(Some(0.0), Some(1.0));
    for _ in 0..100 {
        let value = log_normal.next();
        assert!(value > 0.0 && value.is_finite(), "log-normal: {}", value);
    }

    // The exponential numbers are the ones of the rate, and so they are all
    // above zero.
    let mut exponential = random::random_exponential(Some(2.0));
    for _ in 0..100 {
        let value = exponential.next();
        assert!(value > 0.0 && value.is_finite(), "exponential: {}", value);
    }

    // The Irwin-Hall numbers are the sum of so many numbers in [0, 1), and
    // the Bates ones are that sum over the number of them.
    let mut irwin_hall = random::random_irwin_hall(Some(5.0));
    let mut bates = random::random_bates(Some(5.0));
    for _ in 0..100 {
        let value = irwin_hall.next();
        assert!((0.0..=5.0).contains(&value), "irwin-hall: {}", value);
        let value = bates.next();
        assert!((0.0..=1.0).contains(&value), "bates: {}", value);
    }
}
