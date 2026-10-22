//! The port of d3-dispatch, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-dispatch under
//! Node — the same code the browser runs — and writing down what it
//! produced. The numbers are written down as their bits, for a number
//! written in text and read back may not be the same number.
//!
//! The fixture holds the steps of each scenario as well as what came of
//! them, so that the test replays the very same steps the JavaScript was
//! given.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use d3::dispatch::{dispatch, Dispatch, OBJECT_PROTOTYPE_NAMES};
use serde_json::Value as Json;

/// The fixture, which the oracle wrote.
fn fixture() -> Json {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/dispatch.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The scenarios of the fixture, by name.
fn scenarios() -> Vec<(String, Json)> {
    let fixture = fixture();
    let table = fixture["scenarios"].as_object().expect("the scenarios");
    table
        .iter()
        .map(|(name, scenario)| (name.clone(), scenario.clone()))
        .collect()
}

/// The names of the scenarios, which the tests below go through.
const SCENARIOS: [&str; 12] = [
    "the types are refused",
    "a callback is called",
    "the callbacks are called in order",
    "the callback is given back",
    "a callback is taken away",
    "a named callback is taken away",
    "several types at once",
    "the copy is a copy",
    "the unknown type",
    "the arguments and the context",
    "the whitespace of a typename",
    "the typename with nothing in it",
];

/// The log of a scenario, which the fixture says the JavaScript gave.
fn check(name: &str) {
    let fixture = fixture();
    let scenario = fixture["scenarios"]
        .get(name)
        .unwrap_or_else(|| panic!("the scenario {}", name));
    let steps = scenario["steps"].as_array().expect("the steps");
    let expected: Vec<String> = serde_json::from_value(scenario["log"].clone()).expect("the log");
    let log = run(steps);
    assert_eq!(log, expected, "the scenario {}", name);
}

/// The object the dispatch is called with, which is the JavaScript's `that`.
enum That {
    /// The JavaScript's `null`.
    Null,
    /// The JavaScript's object with a tag in it.
    Tagged(String),
}

/// An argument of a dispatch.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    /// A number, which the fixture holds as its bits.
    Number(f64),
    /// A string.
    Text(String),
}

/// The reading of an argument of a step.
fn value(json: &Json) -> Value {
    match json["bits"].as_str() {
        Some(bits) => Value::Number(f64::from_bits(
            u64::from_str_radix(bits, 16).expect("the bits of a number"),
        )),
        None => Value::Text(json["text"].as_str().expect("the text").to_owned()),
    }
}

/// The description of an argument, which the oracle's own is the same as.
fn describe(value: &Value) -> String {
    match value {
        Value::Number(number) => format!("n:{:016x}", number.to_bits()),
        Value::Text(text) => format!("s:{text}"),
    }
}

/// The run of the steps of a scenario.
struct Run {
    dispatches: HashMap<String, Dispatch<Value>>,
    current: Option<String>,
    log: Rc<RefCell<Vec<String>>>,
}

impl Run {
    /// A line is written into the log.
    fn say(&self, line: String) {
        self.log.borrow_mut().push(line);
    }

    /// The dispatch of the run, which the steps work on.
    fn current(&self) -> &Dispatch<Value> {
        let name = self.current.as_ref().expect("the dispatch of the run");
        self.dispatches
            .get(name)
            .unwrap_or_else(|| panic!("the dispatch {}", name))
    }

    /// The dispatch of the run, which the steps change.
    fn current_mut(&mut self) -> &mut Dispatch<Value> {
        let name = self.current.clone().expect("the dispatch of the run");
        self.dispatches
            .get_mut(&name)
            .unwrap_or_else(|| panic!("the dispatch {}", name))
    }

    /// A step is done.
    fn step(&mut self, step: &Json) {
        let what = step["do"].as_str().expect("the step");
        match what {
            "dispatch" => {
                let name = step["as"].as_str().expect("the name").to_owned();
                let types: Vec<&str> = step["types"]
                    .as_array()
                    .expect("the types")
                    .iter()
                    .map(|name| name.as_str().expect("a type"))
                    .collect();
                match dispatch::<Value>(&types) {
                    Ok(made) => {
                        self.dispatches.insert(name.clone(), made);
                        self.current = Some(name.clone());
                        self.say(format!("dispatch {name}"));
                    }
                    Err(error) => self.say(format!("dispatch {name} ERROR {error}")),
                }
            }
            "on" => {
                let typename = step["typename"].as_str().expect("the typename").to_owned();
                let callback = step["callback"].as_str().expect("the callback").to_owned();
                let log = Rc::clone(&self.log);
                let name = callback.clone();
                let made = self.current_mut().on(&typename, move |that, arguments| {
                    let mut line = format!("{name}(");
                    line.push_str(&match that.downcast_mut::<That>() {
                        Some(That::Null) => "t:null".to_owned(),
                        Some(That::Tagged(tag)) => format!("t:{tag}"),
                        None => "t:unknown".to_owned(),
                    });
                    for argument in arguments {
                        line.push(',');
                        line.push_str(&describe(argument));
                    }
                    line.push(')');
                    log.borrow_mut().push(line);
                });
                match made {
                    Ok(_) => self.say(format!("on {typename} {callback}")),
                    Err(error) => self.say(format!("on {typename} ERROR {error}")),
                }
            }
            "on_get" => {
                let typename = step["typename"].as_str().expect("the typename").to_owned();
                match self.current().on_get(&typename) {
                    Ok(Some(callback)) => {
                        self.say(format!("get {typename} found"));
                        // The callback that was found is called, so that the
                        // log says which one it was.
                        callback(&mut That::Null, &[]);
                    }
                    Ok(None) => self.say(format!("get {typename} none")),
                    Err(error) => self.say(format!("get {typename} ERROR {error}")),
                }
            }
            "on_null" => {
                let typename = step["typename"].as_str().expect("the typename").to_owned();
                match self.current_mut().on_null(&typename) {
                    Ok(_) => self.say(format!("remove {typename}")),
                    Err(error) => self.say(format!("remove {typename} ERROR {error}")),
                }
            }
            "call" | "apply" => {
                let typename = step["typename"].as_str().expect("the typename").to_owned();
                let mut that = match step["that"].as_str() {
                    Some(tag) => That::Tagged(tag.to_owned()),
                    None => That::Null,
                };
                let arguments: Vec<Value> = step["args"]
                    .as_array()
                    .expect("the arguments")
                    .iter()
                    .map(value)
                    .collect();
                let verb = if what == "call" { "call" } else { "apply" };
                self.say(format!("{verb} {typename}"));
                let made = if what == "call" {
                    self.current_mut().call(&typename, &mut that, &arguments)
                } else {
                    self.current_mut().apply(&typename, &mut that, &arguments)
                };
                if let Err(error) = made {
                    self.say(format!("{verb} {typename} ERROR {error}"));
                }
            }
            "copy" => {
                let name = step["as"].as_str().expect("the name").to_owned();
                let from = step["from"]
                    .as_str()
                    .expect("the name it is from")
                    .to_owned();
                let copy = self
                    .dispatches
                    .get(&from)
                    .unwrap_or_else(|| panic!("the dispatch {}", from))
                    .copy();
                self.dispatches.insert(name.clone(), copy);
                self.say(format!("copy {name}"));
            }
            "use" => {
                let name = step["as"].as_str().expect("the name").to_owned();
                self.current = Some(name.clone());
                self.say(format!("use {name}"));
            }
            other => panic!("the step {} is not known", other),
        }
    }
}

/// The run of the steps of a scenario, which gives back the log.
fn run(steps: &[Json]) -> Vec<String> {
    let mut run = Run {
        dispatches: HashMap::new(),
        current: None,
        log: Rc::new(RefCell::new(Vec::new())),
    };
    for step in steps {
        run.step(step);
    }
    let log = run.log.borrow().clone();
    log
}

#[test]
fn the_names_of_the_prototype_are_the_javascripts() {
    let fixture = fixture();
    let expected: Vec<String> =
        serde_json::from_value(fixture["prototypeNames"].clone()).expect("the names");
    assert_eq!(expected, OBJECT_PROTOTYPE_NAMES.to_vec());
}

#[test]
fn the_types_are_refused_as_the_javascript_refuses_them() {
    check("the types are refused");
}

#[test]
fn a_callback_is_called_as_the_javascript_calls_it() {
    check("a callback is called");
}

#[test]
fn the_callbacks_are_called_in_the_javascripts_order() {
    check("the callbacks are called in order");
}

#[test]
fn the_callback_is_given_back_as_the_javascript_gives_it_back() {
    check("the callback is given back");
}

#[test]
fn a_callback_is_taken_away_as_the_javascript_takes_it_away() {
    check("a callback is taken away");
}

#[test]
fn a_named_callback_is_taken_away_as_the_javascript_takes_it_away() {
    check("a named callback is taken away");
}

#[test]
fn several_types_are_set_at_once_as_the_javascript_sets_them() {
    check("several types at once");
}

#[test]
fn the_copy_is_a_copy_as_the_javascripts_is() {
    check("the copy is a copy");
}

#[test]
fn the_unknown_type_is_an_error_as_in_the_javascript() {
    check("the unknown type");
}

#[test]
fn the_arguments_and_the_context_are_the_javascripts() {
    check("the arguments and the context");
}

#[test]
fn the_whitespace_of_a_typename_is_the_javascripts() {
    check("the whitespace of a typename");
}

#[test]
fn the_typename_with_nothing_in_it_is_the_javascripts() {
    check("the typename with nothing in it");
}

#[test]
fn every_scenario_of_the_fixture_is_checked() {
    let names: Vec<String> = scenarios().into_iter().map(|(name, _)| name).collect();
    let mut expected: Vec<&str> = SCENARIOS.to_vec();
    expected.sort_unstable();
    let mut names = names;
    names.sort();
    assert_eq!(names, expected);
}

#[test]
fn the_copy_calls_the_same_callback_as_the_one_it_was_copied_from() {
    // The JavaScript's `copy` copies the array of the callbacks and keeps
    // the functions themselves, so the copy and the one it was copied from
    // call the same function ✓.
    let mut original = dispatch::<Value>(&["start"]).expect("the dispatch");
    original
        .on("start", |_that, _arguments| {})
        .expect("the callback");
    let copy = original.copy();
    let mine = original
        .on_get("start")
        .expect("the callback of the type")
        .expect("the callback");
    let theirs = copy
        .on_get("start")
        .expect("the callback of the type")
        .expect("the callback");
    assert!(Rc::ptr_eq(&mine, &theirs));
}

#[test]
fn the_errors_say_what_the_javascript_says() {
    assert_eq!(
        dispatch::<Value>(&["start", "start"])
            .unwrap_err()
            .to_string(),
        "illegal type: start"
    );
    assert_eq!(
        dispatch::<Value>(&[""]).unwrap_err().to_string(),
        "illegal type: "
    );
    let made = dispatch::<Value>(&["start"]).expect("the dispatch");
    let error = match made.on_get("nope") {
        Ok(_) => panic!("the type is not unknown"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "unknown type: nope");
    assert_eq!(
        made.call("nope", &mut That::Null, &[])
            .unwrap_err()
            .to_string(),
        "unknown type: nope"
    );
}
