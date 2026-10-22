//! The port of d3-timer, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-timer under Node —
//! the same code the browser runs — and writing down what it produced. The
//! numbers are written down as their bits, for a number written in text and
//! read back may not be the same number.
//!
//! The fixture holds the steps of each scenario as well as what came of
//! them, so that the test replays the very same steps the JavaScript was
//! given.
//!
//! The JavaScript's clock is `performance.now` and its event loop is the
//! browser's own, and neither of them can be told what to do from here. The
//! test therefore gives the port a clock of its own and a loop of its own,
//! the same two the oracle gave the JavaScript.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use d3::timer::{Clock, EventLoop, Timer, Timers};
use serde_json::Value as Json;

/// The fixture, which the oracle wrote.
fn fixture() -> Json {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/timer.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The number a fixture holds, which is written down as its bits.
fn number(value: &Json) -> f64 {
    let text = value["bits"].as_str().expect("the bits of a number");
    let mut bits = 0u64;
    for index in 0..16 {
        let digit = u8::from_str_radix(&text[index..index + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    f64::from_bits(bits)
}

/// The number of a step, which may be nothing at all.
fn maybe_number(value: &Json) -> Option<f64> {
    if value.is_null() {
        None
    } else {
        Some(number(value))
    }
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
const SCENARIOS: [&str; 18] = [
    "the clock is read",
    "a timer fires when its time comes",
    "a timer with a time of its own",
    "a timer with no delay",
    "a timer whose time has passed",
    "a timer that is stopped does not fire",
    "a timer that is stopped and started again",
    "a timeout fires once",
    "a timeout with no delay",
    "an interval goes on firing",
    "an interval with no delay",
    "the timers are called in the order they were made in",
    "a timer that is started again",
    "a callback that stops another timer",
    "the flush with nothing to do",
    "the timer that is due within the frame",
    "the timer that is due after the frame",
    "the clock's skew",
];

/// A timeout of the loop of the test, which is waiting to be run.
struct Pending {
    /// The time the timeout is due at.
    due: f64,
    /// The JavaScript's `wake`.
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

/// An interval of the loop of the test, which is running.
struct Running {
    /// The time between the turns of the interval.
    period: f64,
    /// The time the interval last ran at.
    last: f64,
    /// The JavaScript's `poke`.
    callback: Rc<dyn Fn()>,
}

/// The state of the loop of the test, which stands in for the browser's own
/// loop and for the JavaScript's `performance.now`.
#[derive(Default)]
struct LoopState {
    /// The clock, in milliseconds.
    clock: f64,
    /// The timeout that is pending.
    timeout: Option<Pending>,
    /// The interval that is running.
    interval: Option<Running>,
    /// The frames that were asked for.
    frames: Vec<Box<dyn FnOnce()>>,
    /// The log of the run.
    log: Vec<String>,
}

/// The loop of the test, which stands in for the browser's own loop.
struct FakeLoop {
    /// The state the loop and the run share.
    state: Rc<RefCell<LoopState>>,
}

impl EventLoop for FakeLoop {
    /// The JavaScript's `setTimeout(f, delay)`.
    fn set_timeout(&mut self, delay: f64, callback: Box<dyn FnOnce()>) {
        let mut state = self.state.borrow_mut();
        state
            .log
            .push(format!("setTimeout n:{:016x}", delay.to_bits()));
        let due = state.clock + delay;
        state.timeout = Some(Pending {
            due,
            callback: RefCell::new(Some(callback)),
        });
    }

    /// The JavaScript's `clearTimeout(handle)`.
    fn clear_timeout(&mut self) {
        let mut state = self.state.borrow_mut();
        state.log.push("clearTimeout".to_owned());
        state.timeout = None;
    }

    /// The JavaScript's `setInterval(f, delay)`.
    fn set_interval(&mut self, delay: f64, callback: Rc<dyn Fn()>) {
        let mut state = self.state.borrow_mut();
        state
            .log
            .push(format!("setInterval n:{:016x}", delay.to_bits()));
        state.interval = Some(Running {
            period: delay,
            last: state.clock,
            callback,
        });
    }

    /// The JavaScript's `clearInterval(handle)`.
    fn clear_interval(&mut self) {
        let mut state = self.state.borrow_mut();
        state.log.push("clearInterval".to_owned());
        state.interval = None;
    }

    /// The JavaScript's `setFrame(f)`.
    fn set_frame(&mut self, callback: Box<dyn FnOnce()>) {
        let mut state = self.state.borrow_mut();
        state.log.push("ask frame".to_owned());
        state.frames.push(callback);
    }
}

/// The clock of the test, which stands in for the JavaScript's
/// `performance.now`.
struct FakeClock {
    /// The state the clock and the run share.
    state: Rc<RefCell<LoopState>>,
}

impl Clock for FakeClock {
    /// The JavaScript's `clock.now()`.
    fn now(&self) -> f64 {
        self.state.borrow().clock
    }
}

/// The run of the steps of a scenario.
struct Run {
    /// The JavaScript's module.
    timers: Timers,
    /// The state of the loop of the test.
    state: Rc<RefCell<LoopState>>,
    /// The timers that were made, by their number.
    made: Rc<RefCell<HashMap<u64, Timer>>>,
}

impl Run {
    /// A line is written into the log.
    fn say(&self, line: String) {
        self.state.borrow_mut().log.push(line);
    }

    /// A callback of a timer, which says what it was called with.
    fn callback(&self, id: u64, stops: Option<u64>) -> impl Fn(f64) + 'static {
        let state = Rc::clone(&self.state);
        let made = Rc::clone(&self.made);
        move |elapsed: f64| {
            state
                .borrow_mut()
                .log
                .push(format!("call {id} n:{:016x}", elapsed.to_bits()));
            if let Some(stops) = stops {
                let other = made.borrow().get(&stops).cloned();
                if let Some(other) = other {
                    other.stop();
                }
            }
        }
    }

    /// A step is done.
    fn step(&mut self, step: &Json) {
        let what = step["do"].as_str().expect("the step");
        match what {
            "set" => {
                let clock = number(&step["clock"]);
                self.say(format!("set n:{:016x}", clock.to_bits()));
                self.state.borrow_mut().clock = clock;
            }
            "advance" => {
                let by = number(&step["by"]);
                self.say(format!("advance n:{:016x}", by.to_bits()));
                self.state.borrow_mut().clock += by;
                // The timeout, if its time has come.
                let timeout = {
                    let mut state = self.state.borrow_mut();
                    match &state.timeout {
                        Some(pending) if pending.due <= state.clock => state.timeout.take(),
                        _ => None,
                    }
                };
                if let Some(pending) = timeout {
                    let callback = pending.callback.borrow_mut().take();
                    if let Some(callback) = callback {
                        callback();
                    }
                }
                // The interval, if a whole period of it has gone by.
                let interval = {
                    let mut state = self.state.borrow_mut();
                    let clock = state.clock;
                    let due = match &state.interval {
                        Some(running) => clock - running.last >= running.period,
                        None => false,
                    };
                    if due {
                        if let Some(running) = state.interval.as_mut() {
                            running.last = clock;
                            Some(Rc::clone(&running.callback))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(interval) = interval {
                    interval();
                }
            }
            "timer" | "timeout" | "interval" => {
                let id = step["id"].as_u64().expect("the id");
                let stops = step.get("stops").and_then(|stops| stops.as_u64());
                self.say(format!("{what} {id}"));
                let delay = maybe_number(&step["delay"]);
                let time = maybe_number(&step["time"]);
                let callback = self.callback(id, stops);
                let timer = match what {
                    "timer" => self.timers.timer(callback, delay, time),
                    "timeout" => self.timers.timeout(callback, delay, time),
                    _ => self.timers.interval(callback, delay, time),
                };
                self.made.borrow_mut().insert(id, timer);
            }
            "restart" => {
                let id = step["id"].as_u64().expect("the id");
                let stops = step.get("stops").and_then(|stops| stops.as_u64());
                self.say(format!("restart {id}"));
                let delay = maybe_number(&step["delay"]);
                let time = maybe_number(&step["time"]);
                let callback = self.callback(id, stops);
                let timer = self.made.borrow().get(&id).cloned().expect("the timer");
                timer.restart(callback, delay, time);
            }
            "stop" => {
                let id = step["id"].as_u64().expect("the id");
                self.say(format!("stop {id}"));
                let timer = self.made.borrow().get(&id).cloned().expect("the timer");
                timer.stop();
            }
            "flush" => {
                self.say("flush".to_owned());
                self.timers.timer_flush();
            }
            "frame" => {
                self.say("frame".to_owned());
                // The browser's own frames run every callback that was asked
                // for before the frame came.
                let frames: Vec<Box<dyn FnOnce()>> = {
                    let mut state = self.state.borrow_mut();
                    std::mem::take(&mut state.frames)
                };
                for frame in frames {
                    frame();
                }
            }
            "now" => {
                let value = self.timers.now();
                self.say(format!("now n:{:016x}", value.to_bits()));
            }
            other => panic!("the step {} is not known", other),
        }
    }
}

/// The run of the steps of a scenario, which gives back the log.
fn run(steps: &[Json]) -> Vec<String> {
    let state = Rc::new(RefCell::new(LoopState::default()));
    let timers = Timers::new(
        FakeClock {
            state: Rc::clone(&state),
        },
        FakeLoop {
            state: Rc::clone(&state),
        },
    );
    let mut run = Run {
        timers,
        state,
        made: Rc::new(RefCell::new(HashMap::new())),
    };
    for step in steps {
        run.step(step);
    }
    let log = run.state.borrow().log.clone();
    log
}

/// The log of a scenario, against the log the JavaScript gave.
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

#[test]
fn the_clock_is_read_as_the_javascript_reads_it() {
    check("the clock is read");
}

#[test]
fn a_timer_fires_as_the_javascript_fires_it() {
    check("a timer fires when its time comes");
}

#[test]
fn a_timer_with_a_time_of_its_own_is_as_the_javascript_has_it() {
    check("a timer with a time of its own");
}

#[test]
fn a_timer_with_no_delay_is_as_the_javascript_has_it() {
    check("a timer with no delay");
}

#[test]
fn a_timer_whose_time_has_passed_is_as_the_javascript_has_it() {
    check("a timer whose time has passed");
}

#[test]
fn a_timer_that_is_stopped_does_not_fire_as_in_the_javascript() {
    check("a timer that is stopped does not fire");
}

#[test]
fn a_timer_that_is_stopped_and_started_again_is_as_the_javascript_has_it() {
    check("a timer that is stopped and started again");
}

#[test]
fn a_timeout_fires_as_the_javascript_fires_it() {
    check("a timeout fires once");
}

#[test]
fn a_timeout_with_no_delay_is_as_the_javascript_has_it() {
    check("a timeout with no delay");
}

#[test]
fn an_interval_goes_on_firing_as_the_javascript_does() {
    check("an interval goes on firing");
}

#[test]
fn an_interval_with_no_delay_is_as_the_javascript_has_it() {
    check("an interval with no delay");
}

#[test]
fn the_timers_are_called_in_the_javascripts_order() {
    check("the timers are called in the order they were made in");
}

#[test]
fn a_timer_that_is_started_again_is_as_the_javascript_has_it() {
    check("a timer that is started again");
}

#[test]
fn a_callback_that_stops_another_timer_is_as_the_javascript_has_it() {
    check("a callback that stops another timer");
}

#[test]
fn the_flush_with_nothing_to_do_is_as_the_javascript_has_it() {
    check("the flush with nothing to do");
}

#[test]
fn the_timer_that_is_due_within_the_frame_is_as_the_javascript_has_it() {
    check("the timer that is due within the frame");
}

#[test]
fn the_timer_that_is_due_after_the_frame_is_as_the_javascript_has_it() {
    check("the timer that is due after the frame");
}

#[test]
fn the_clocks_skew_is_as_the_javascript_has_it() {
    check("the clock's skew");
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
fn the_scenarios_of_the_oracle_are_checked() {
    // The oracle makes a hundred and fifty scenarios of its own, out of
    // its own seed, and the port is run through them as well.
    let fixture = fixture();
    let fuzz = fixture["fuzz"]
        .as_array()
        .expect("the scenarios of the oracle");
    for (index, scenario) in fuzz.iter().enumerate() {
        let steps = scenario["steps"].as_array().expect("the steps");
        let expected: Vec<String> =
            serde_json::from_value(scenario["log"].clone()).expect("the log");
        let log = run(steps);
        assert_eq!(log, expected, "the scenario {} of the oracle", index);
    }
}

#[test]
fn the_clock_of_the_system_goes_on() {
    // The JavaScript's `performance.now` counts the time that has passed
    // since the page was loaded, and the port's own clock counts the time
    // that has passed since it was made ✓.
    let clock = d3::timer::SystemClock::new();
    let first = clock.now();
    assert!(first >= 0.0, "the clock is {}", first);
    std::thread::sleep(std::time::Duration::from_millis(2));
    let second = clock.now();
    assert!(second > first, "the clock is {} and then {}", first, second);
}

#[test]
fn a_clock_that_gives_a_number_that_is_not_a_number_is_read_again() {
    // The JavaScript's `clockNow || (...)` is the JavaScript's own
    // truthiness, and a number that is not a number is false, so the clock
    // is read again and again, and the JavaScript does the same ✓.
    struct Counting {
        /// The number of times the clock has been read.
        reads: Rc<Cell<u32>>,
    }

    impl Clock for Counting {
        /// The JavaScript's `clock.now()`, which gives a number that is not
        /// a number.
        fn now(&self) -> f64 {
            self.reads.set(self.reads.get() + 1);
            f64::NAN
        }
    }

    let state = Rc::new(RefCell::new(LoopState::default()));
    let reads = Rc::new(Cell::new(0));
    let timers = Timers::new(
        Counting {
            reads: Rc::clone(&reads),
        },
        FakeLoop { state },
    );
    assert!(timers.now().is_nan());
    assert!(timers.now().is_nan());
    assert_eq!(reads.get(), 2);
}

#[test]
fn the_timer_of_the_javascript_is_the_ports_own() {
    // The JavaScript's `Timer` is made with `new Timer` and it is given a
    // call, a time and a link to the next timer of the list, and the port's
    // is the same ✓.
    let state = Rc::new(RefCell::new(LoopState::default()));
    let timers = Timers::new(
        FakeClock {
            state: Rc::clone(&state),
        },
        FakeLoop {
            state: Rc::clone(&state),
        },
    );

    // A timer that is made and not started waits for nothing.
    let waiting = timers.new_timer();
    assert!(format!("{waiting:?}").contains("waiting: false"));

    // A timer that is started waits, and its time is the time it was given
    // and the delay it was given ✓.
    let started = timers.new_timer();
    started.restart(|_elapsed| {}, Some(100.0), Some(1000.0));
    assert!(format!("{started:?}").contains("waiting: true"));
    assert!(format!("{started:?}").contains("time: 1100.0"));

    // A timer that is stopped waits for nothing, and its time is the
    // infinite ✓.
    let stopped = timers.new_timer();
    stopped.restart(|_elapsed| {}, Some(100.0), Some(1000.0));
    stopped.stop();
    assert!(format!("{stopped:?}").contains("waiting: false"));
    assert!(format!("{stopped:?}").contains("time: inf"));
}

#[test]
fn a_timer_that_stops_itself_is_not_called_again() {
    // The JavaScript's `timerFlush` walks the list of the timers, and a
    // timer that stops itself while it is being walked is not called
    // again, and the walk carries on ✓.
    let state = Rc::new(RefCell::new(LoopState::default()));
    let timers = Timers::new(
        FakeClock {
            state: Rc::clone(&state),
        },
        FakeLoop {
            state: Rc::clone(&state),
        },
    );

    let calls = Rc::new(RefCell::new(Vec::new()));
    let mine = Rc::clone(&calls);
    let holder: Rc<RefCell<Option<Timer>>> = Rc::new(RefCell::new(None));
    let stopper = Rc::clone(&holder);
    let timer = timers.timer(
        move |elapsed| {
            mine.borrow_mut().push(elapsed);
            let timer = stopper.borrow().clone();
            if let Some(timer) = timer {
                timer.stop();
            }
        },
        Some(0.0),
        Some(0.0),
    );
    *holder.borrow_mut() = Some(timer);

    timers.timer_flush();
    assert_eq!(calls.borrow().len(), 1);
    timers.timer_flush();
    assert_eq!(calls.borrow().len(), 1);
}
