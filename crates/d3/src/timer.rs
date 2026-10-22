//! A port of **d3-timer**, from the JavaScript of the same name.
//!
//! Translated from `d3-timer/src/*.js`, version 1.0.10, of d3 5.16.0. The
//! JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! # What the JavaScript is
//!
//! d3-timer is the queue of the timers of a program. A *timer* calls a
//! function again and again, a *timeout* calls it once, and an *interval*
//! calls it again and again with the time it has been running added up.
//! Every one of them is given the time that has passed since the timer was
//! started, in milliseconds ✓.
//!
//! ```js
//! var t = timer(function(elapsed) { console.log(elapsed); }, 100);
//! t.stop();
//! ```
//!
//! # The clock and the loop
//!
//! The JavaScript's clock is `performance.now`, and its event loop is the
//! browser's own: the JavaScript asks the browser for a timeout, for an
//! interval and for an animation frame, and the browser's loop runs them.
//! The port has neither a browser nor a clock of its own, so it asks for
//! both: [`Clock`] is the JavaScript's `performance.now` ✓, and
//! [`EventLoop`] is the browser's own loop ✓.
//!
//! The JavaScript's arithmetic is kept exactly: the `delay` is added to the
//! time the timer is given ✓, the time a callback is given is the time that
//! has passed since the timer was due ✓ — and the timeout and the interval
//! are given the time that has passed since they were made ✓ — and a timer
//! that has been stopped does not fire ✓.
//!
//! # The shape of the port
//!
//! The JavaScript's timers are the module's own: one list of the timers that
//! are waiting, one clock and one loop. The port's [`Timers`] is that
//! module, and a [`Timer`] is one of the timers in its list ✓.
//!
//! The JavaScript's timer holds its own place in the list, and the port's
//! timer is behind a reference count, so that a timer may be started again
//! from inside its own callback ✓, which the JavaScript's interval does ✓.
//!
//! # What the port cannot do
//!
//! The JavaScript's `finally` in its `wake` runs when a callback throws.
//! The port's does not, for a panic is not a thing the port can catch, and
//! the timers that are waiting are left as they are ✓.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Instant;

use crate::js;

/// The JavaScript's `pokeDelay`, which is how often the clock's skew is
/// looked at.
const POKE_DELAY: f64 = 1000.0;

/// The JavaScript's clock, which is `performance.now` where the browser has
/// it and `Date.now` where it has not.
///
/// The JavaScript's clock is a number of milliseconds, and the port's is the
/// same ✓.
pub trait Clock {
    /// The JavaScript's `clock.now()`.
    fn now(&self) -> f64;
}

/// The JavaScript's clock of the system's own, which is `performance.now`.
///
/// The JavaScript's `performance.now` counts from the moment the page was
/// loaded, and the port's counts from the moment the clock was made ✓.
#[derive(Debug)]
pub struct SystemClock {
    /// The moment the clock was made, which the time is counted from.
    start: Instant,
}

impl SystemClock {
    /// A clock that starts now.
    pub fn new() -> Self {
        SystemClock {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    /// A clock that starts now.
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    /// The time that has passed since the clock was made, in milliseconds.
    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

/// The JavaScript's event loop, which is the browser's own.
///
/// The JavaScript's timers ask the browser for a timeout, for an interval
/// and for an animation frame, and the browser's loop runs them. The port
/// has no browser, so it asks a loop of the program's own ✓.
pub trait EventLoop {
    /// The JavaScript's `setTimeout(f, delay)`, which runs `f` after
    /// `delay` milliseconds.
    fn set_timeout(&mut self, delay: f64, callback: Box<dyn FnOnce()>);

    /// The JavaScript's `clearTimeout(handle)`, which does not run the
    /// timeout that was set.
    fn clear_timeout(&mut self);

    /// The JavaScript's `setInterval(f, delay)`, which runs `f` every
    /// `delay` milliseconds.
    ///
    /// The JavaScript's interval is run again and again, so the port's is
    /// behind a reference count, as the callbacks of the crate's own
    /// dispatch are ✓.
    fn set_interval(&mut self, delay: f64, callback: Rc<dyn Fn()>);

    /// The JavaScript's `clearInterval(handle)`, which does not run the
    /// interval that was set.
    fn clear_interval(&mut self);

    /// The JavaScript's `setFrame(f)`, which is `requestAnimationFrame` where
    /// the browser has one and `setTimeout(f, 17)` where it has not.
    ///
    /// The JavaScript's `now` asks for a frame of its own, to take the
    /// cached time away, and its `sleep` asks for a frame to wake on ✓.
    fn set_frame(&mut self, callback: Box<dyn FnOnce()>);
}

/// The JavaScript's module: the timers that are waiting, the clock they
/// wait on and the loop that wakes them.
///
/// The JavaScript's module is a set of module-level variables, and every
/// timer of the program is in it. The port's [`Timers`] is that module, and
/// a copy of it is a handle on the same module ✓.
pub struct Timers {
    shared: Rc<Shared>,
}

impl Timers {
    /// The JavaScript's module, with a clock and an event loop of the
    /// program's own.
    pub fn new(clock: impl Clock + 'static, event_loop: impl EventLoop + 'static) -> Self {
        Timers {
            shared: Rc::new(Shared {
                state: RefCell::new(State {
                    frame: 0,
                    timeout: false,
                    interval: false,
                    task_head: None,
                    task_tail: None,
                    clock_last: 0.0,
                    clock_now: 0.0,
                    clock_skew: 0.0,
                }),
                clock: Box::new(clock),
                event_loop: RefCell::new(Box::new(event_loop)),
            }),
        }
    }

    /// The JavaScript's `now`, which gives the time of the frame that is
    /// running.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// export function now() {
    ///   return clockNow || (setFrame(clearNow), clockNow = clock.now() + clockSkew);
    /// }
    /// ```
    ///
    /// The JavaScript keeps the time of the frame until the next frame
    /// comes, and asks the browser for that frame here ✓. The port keeps it
    /// in the same way, and the frame is the program's own ✓.
    pub fn now(&self) -> f64 {
        let clock_now = self.clock_now();
        // The JavaScript's `clockNow || (...)`, which is the JavaScript's
        // own truthiness: a zero and a number that is not a number are both
        // false ✓.
        if js::truthy(clock_now) {
            return clock_now;
        }
        // The JavaScript's `setFrame(clearNow)`, which takes the cached time
        // away when the next frame comes.
        self.shared
            .event_loop
            .borrow_mut()
            .set_frame(Box::new(clear_now_closure(&self.shared)));
        let value = self.read_clock() + self.clock_skew();
        self.shared.state.borrow_mut().clock_now = value;
        value
    }

    /// The JavaScript's `timer`, which calls a function again and again.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// export function timer(callback, delay, time) {
    ///   var t = new Timer;
    ///   t.restart(callback, delay, time);
    ///   return t;
    /// }
    /// ```
    pub fn timer(
        &self,
        callback: impl Fn(f64) + 'static,
        delay: Option<f64>,
        time: Option<f64>,
    ) -> Timer {
        let timer = self.new_timer();
        timer.restart(callback, delay, time);
        timer
    }

    /// The JavaScript's `timeout`, which calls a function once.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// export default function(callback, delay, time) {
    ///   var t = new Timer;
    ///   delay = delay == null ? 0 : +delay;
    ///   t.restart(function(elapsed) {
    ///     t.stop();
    ///     callback(elapsed + delay);
    ///   }, delay, time);
    ///   return t;
    /// }
    /// ```
    ///
    /// The JavaScript's timeout is given the time that has passed since it
    /// was started, and not the time that has passed since it was due ✓,
    /// which is what the `delay` is for.
    pub fn timeout(
        &self,
        callback: impl Fn(f64) + 'static,
        delay: Option<f64>,
        time: Option<f64>,
    ) -> Timer {
        let timer = self.new_timer();
        // The JavaScript's `delay = delay == null ? 0 : +delay;`
        let delay = delay.unwrap_or(0.0);
        let callback: Rc<dyn Fn(f64)> = Rc::new(callback);
        let shared = Rc::downgrade(&self.shared);
        let task = Rc::downgrade(&timer.task);
        let call: Rc<dyn Fn(f64)> = Rc::new(move |elapsed: f64| {
            // The JavaScript's `t.stop();`, which the timeout does before it
            // calls back, so that it does not fire again ✓.
            if let (Some(shared), Some(task)) = (shared.upgrade(), task.upgrade()) {
                stop_task(&shared, &task);
            }
            // The JavaScript's `callback(elapsed + delay);`
            callback(elapsed + delay);
        });
        timer.restart_with(call, Some(delay), time);
        timer
    }

    /// The JavaScript's `interval`, which calls a function again and again
    /// with the time it has been running added up.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// export default function(callback, delay, time) {
    ///   var t = new Timer, total = delay;
    ///   if (delay == null) return t.restart(callback, delay, time), t;
    ///   delay = +delay, time = time == null ? now() : +time;
    ///   t.restart(function tick(elapsed) {
    ///     elapsed += total;
    ///     t.restart(tick, total += delay, time);
    ///     callback(elapsed);
    ///   }, delay, time);
    ///   return t;
    /// }
    /// ```
    ///
    /// The JavaScript's interval counts the time from the moment it was
    /// made, and not from the moment it was due, so the time it gives is
    /// the whole of it ✓. The port's counts the same way ✓.
    pub fn interval(
        &self,
        callback: impl Fn(f64) + 'static,
        delay: Option<f64>,
        time: Option<f64>,
    ) -> Timer {
        let timer = self.new_timer();
        // The JavaScript's `if (delay == null) return t.restart(callback, delay, time), t;`
        let Some(delay) = delay else {
            timer.restart(callback, None, time);
            return timer;
        };
        // The JavaScript's `delay = +delay, time = time == null ? now() : +time;`
        let time = Some(time.unwrap_or_else(|| self.now()));
        let callback: Rc<dyn Fn(f64)> = Rc::new(callback);
        // The JavaScript's `total`, which is the delay to begin with and is
        // added to on every turn ✓.
        let total = Rc::new(Cell::new(delay));
        let call = make_tick(
            Rc::downgrade(&self.shared),
            Rc::downgrade(&timer.task),
            total,
            delay,
            time,
            callback,
        );
        timer.restart_with(call, Some(delay), time);
        timer
    }

    /// The JavaScript's `timerFlush`, which calls the timers whose time has
    /// come.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// export function timerFlush() {
    ///   now(); // Get the current time, if not already set.
    ///   ++frame; // Pretend we've set an alarm, if we haven't already.
    ///   var t = taskHead, e;
    ///   while (t) {
    ///     if ((e = clockNow - t._time) >= 0) t._call.call(null, e);
    ///     t = t._next;
    ///   }
    ///   --frame;
    /// }
    /// ```
    ///
    /// The JavaScript walks the list of the timers in the order the timers
    /// were made in, and not in the order of their times ✓, and it reads
    /// the next timer of the list after the callback has run ✓, so that a
    /// callback that starts a timer again is seen ✓.
    pub fn timer_flush(&self) {
        // The JavaScript's `now();` — the time of the frame, if it is not
        // known already.
        self.now();
        // The JavaScript's `++frame;` — the flush pretends that an alarm is
        // set, so that a timer that is started inside it does not ask for a
        // frame of its own ✓.
        self.shared.state.borrow_mut().frame += 1;
        let mut task = self.shared.state.borrow().task_head.clone();
        while let Some(current) = task {
            let (call, time) = {
                let current = current.borrow();
                (current.call.clone(), current.time)
            };
            // The JavaScript's `if ((e = clockNow - t._time) >= 0)`.
            let elapsed = self.clock_now() - time;
            if elapsed >= 0.0 {
                // The JavaScript's `t._call.call(null, e)`. A timer that has
                // been stopped has no call, and its time is the infinite,
                // so the elapsed is the negative infinite and it is not
                // called ✓.
                if let Some(call) = call {
                    call(elapsed);
                }
            }
            // The JavaScript's `t = t._next;`, which is read after the
            // callback has run ✓.
            task = current.borrow().next.clone();
        }
        // The JavaScript's `--frame;`
        self.shared.state.borrow_mut().frame -= 1;
    }

    /// The JavaScript's `new Timer`, which is a timer that is not waiting
    /// for anything yet.
    pub fn new_timer(&self) -> Timer {
        Timer {
            shared: Rc::clone(&self.shared),
            task: Rc::new(RefCell::new(Task {
                call: None,
                time: 0.0,
                next: None,
            })),
        }
    }

    /// The JavaScript's `clockNow`, which is the time of the frame that is
    /// running.
    fn clock_now(&self) -> f64 {
        self.shared.state.borrow().clock_now
    }

    /// The JavaScript's `clock.now()`, which reads the clock of the
    /// program's own.
    fn read_clock(&self) -> f64 {
        self.shared.clock.now()
    }

    /// The JavaScript's `clockSkew`, which is what the clock of the program
    /// is read against.
    fn clock_skew(&self) -> f64 {
        self.shared.state.borrow().clock_skew
    }

    /// The JavaScript's `wake`, which the loop calls when the frame or the
    /// timeout comes.
    fn wake(&self) {
        let reading = self.read_clock();
        {
            let mut state = self.shared.state.borrow_mut();
            // The JavaScript's `clockNow = (clockLast = clock.now()) + clockSkew;`
            state.clock_last = reading;
            state.clock_now = reading + state.clock_skew;
            // The JavaScript's `frame = timeout = 0;`
            state.frame = 0;
            state.timeout = false;
        }
        // The JavaScript's `try { timerFlush(); } finally { frame = 0; nap(); clockNow = 0; }`
        self.timer_flush();
        {
            // The JavaScript's `frame = 0;`
            self.shared.state.borrow_mut().frame = 0;
        }
        // The JavaScript's `nap();`
        self.nap();
        // The JavaScript's `clockNow = 0;`
        self.shared.state.borrow_mut().clock_now = 0.0;
    }

    /// The JavaScript's `poke`, which the loop calls now and again to see
    /// whether the clock has jumped.
    fn poke(&self) {
        let reading = self.read_clock();
        let last = self.shared.state.borrow().clock_last;
        // The JavaScript's `var now = clock.now(), delay = now - clockLast;`
        let delay = reading - last;
        // The JavaScript's `if (delay > pokeDelay) clockSkew -= delay, clockLast = now;`
        if delay > POKE_DELAY {
            let mut state = self.shared.state.borrow_mut();
            state.clock_skew -= delay;
            state.clock_last = reading;
        }
    }

    /// The JavaScript's `nap`, which takes the timers that have stopped out
    /// of the list and waits for the next one.
    fn nap(&self) {
        // The JavaScript's `t0`, which is the last of the timers that are
        // still waiting ✓.
        let mut last: Option<Rc<RefCell<Task>>> = None;
        let mut task = self.shared.state.borrow().task_head.clone();
        // The JavaScript's `time = Infinity`, which is the time of the
        // first of the timers that are waiting ✓.
        let mut time = f64::INFINITY;
        while let Some(current) = task {
            let (waiting, current_time, next) = {
                let current = current.borrow();
                (current.call.is_some(), current.time, current.next.clone())
            };
            if waiting {
                // The JavaScript's `if (time > t1._time) time = t1._time;`
                if time > current_time {
                    time = current_time;
                }
                // The JavaScript's `t0 = t1, t1 = t1._next;`
                last = Some(Rc::clone(&current));
                task = next;
            } else {
                // The JavaScript's `t2 = t1._next, t1._next = null;`
                current.borrow_mut().next = None;
                match &last {
                    // The JavaScript's `t1 = t0 ? t0._next = t2 : taskHead = t2;`
                    Some(previous) => previous.borrow_mut().next = next.clone(),
                    None => self.shared.state.borrow_mut().task_head = next.clone(),
                }
                task = next;
            }
        }
        // The JavaScript's `taskTail = t0;`
        self.shared.state.borrow_mut().task_tail = last;
        // The JavaScript's `sleep(time);`
        self.sleep(Some(time));
    }

    /// The JavaScript's `sleep`, which asks the loop for the next frame or
    /// the next timeout.
    fn sleep(&self, time: Option<f64>) {
        // The JavaScript's `if (frame) return;` — a frame is pending
        // already, and it will come soon enough ✓.
        let pending = self.shared.state.borrow().frame;
        if pending != 0 {
            return;
        }
        // The JavaScript's `if (timeout) timeout = clearTimeout(timeout);`
        let timeout = self.shared.state.borrow().timeout;
        if timeout {
            self.shared.state.borrow_mut().timeout = false;
            self.shared.event_loop.borrow_mut().clear_timeout();
        }
        // The JavaScript's `var delay = time - clockNow;` — a time that was
        // not there at all is a number that is not a number, and a number
        // that is not a number is not more than twenty-four ✓.
        let delay = match time {
            Some(time) => time - self.clock_now(),
            None => f64::NAN,
        };
        if delay > 24.0 {
            // The JavaScript's `if (time < Infinity) timeout = setTimeout(wake, time - clock.now() - clockSkew);`
            if let Some(time) = time {
                if time < f64::INFINITY {
                    let wait = time - self.read_clock() - self.clock_skew();
                    self.shared
                        .event_loop
                        .borrow_mut()
                        .set_timeout(wait, Box::new(wake_closure(&self.shared)));
                    self.shared.state.borrow_mut().timeout = true;
                }
            }
            // The JavaScript's `if (interval) interval = clearInterval(interval);`
            let interval = self.shared.state.borrow().interval;
            if interval {
                self.shared.state.borrow_mut().interval = false;
                self.shared.event_loop.borrow_mut().clear_interval();
            }
        } else {
            // The JavaScript's `if (!interval) clockLast = clock.now(), interval = setInterval(poke, pokeDelay);`
            let interval = self.shared.state.borrow().interval;
            if !interval {
                let reading = self.read_clock();
                self.shared.state.borrow_mut().clock_last = reading;
                self.shared
                    .event_loop
                    .borrow_mut()
                    .set_interval(POKE_DELAY, poke_closure(&self.shared));
                self.shared.state.borrow_mut().interval = true;
            }
            // The JavaScript's `frame = 1, setFrame(wake);`
            self.shared.state.borrow_mut().frame = 1;
            self.shared
                .event_loop
                .borrow_mut()
                .set_frame(Box::new(wake_closure(&self.shared)));
        }
    }
}

impl Clone for Timers {
    /// The JavaScript's module is one module, and a copy of the port's
    /// timers is a handle on the same module ✓.
    fn clone(&self) -> Self {
        Timers {
            shared: Rc::clone(&self.shared),
        }
    }
}

impl std::fmt::Debug for Timers {
    /// The timers that are waiting, and the clock they wait on.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.shared.state.borrow();
        f.debug_struct("Timers")
            .field("waiting", &state.task_head.is_some())
            .field("clock_now", &state.clock_now)
            .field("clock_skew", &state.clock_skew)
            .finish()
    }
}

/// The JavaScript's `Timer`, which is a timer that is waiting.
///
/// The JavaScript's timer is an object with a call, a time and a link to
/// the next timer of the list, and the port's is the same ✓.
pub struct Timer {
    shared: Rc<Shared>,
    task: Rc<RefCell<Task>>,
}

impl Timer {
    /// The JavaScript's `restart`, which starts the timer again.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// restart: function(callback, delay, time) {
    ///   if (typeof callback !== "function") throw new TypeError("callback is not a function");
    ///   time = (time == null ? now() : +time) + (delay == null ? 0 : +delay);
    ///   if (!this._next && taskTail !== this) {
    ///     if (taskTail) taskTail._next = this;
    ///     else taskHead = this;
    ///     taskTail = this;
    ///   }
    ///   this._call = callback;
    ///   this._time = time;
    ///   sleep();
    /// }
    /// ```
    ///
    /// The JavaScript's `callback is not a function` error cannot happen in
    /// the port, because a closure is a closure ✓.
    ///
    /// The JavaScript's `sleep()` is called with no time at all, and a time
    /// that is not there at all is a number that is not a number, and so the
    /// frame is asked for ✓.
    pub fn restart(&self, callback: impl Fn(f64) + 'static, delay: Option<f64>, time: Option<f64>) {
        self.restart_with(Rc::new(callback), delay, time);
    }

    /// The JavaScript's `stop`, which stops the timer.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// stop: function() {
    ///   if (this._call) {
    ///     this._call = null;
    ///     this._time = Infinity;
    ///     sleep();
    ///   }
    /// }
    /// ```
    ///
    /// The JavaScript's stopped timer has the infinite for its time, and so
    /// it is never due, and it does not fire ✓.
    pub fn stop(&self) {
        stop_task(&self.shared, &self.task);
    }

    /// The JavaScript's `restart`, with a call that is already made.
    fn restart_with(&self, call: Rc<dyn Fn(f64)>, delay: Option<f64>, time: Option<f64>) {
        restart_task(&self.shared, &self.task, call, delay, time);
    }
}

impl Clone for Timer {
    /// The JavaScript's timer is an object, and the port's copy is a handle
    /// on the same timer ✓.
    fn clone(&self) -> Self {
        Timer {
            shared: Rc::clone(&self.shared),
            task: Rc::clone(&self.task),
        }
    }
}

impl std::fmt::Debug for Timer {
    /// The time the timer is due at, and whether it is waiting.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let task = self.task.borrow();
        f.debug_struct("Timer")
            .field("waiting", &task.call.is_some())
            .field("time", &task.time)
            .finish()
    }
}

/// The JavaScript's module-level variables, which every timer of the
/// program shares.
struct State {
    /// The JavaScript's `frame`, which counts the frames that are pending.
    frame: u32,
    /// The JavaScript's `timeout`, which says whether a timeout is pending.
    timeout: bool,
    /// The JavaScript's `interval`, which says whether the interval that
    /// looks at the clock's skew is running.
    interval: bool,
    /// The JavaScript's `taskHead`, the first of the timers that are
    /// waiting.
    task_head: Option<Rc<RefCell<Task>>>,
    /// The JavaScript's `taskTail`, the last of them.
    task_tail: Option<Rc<RefCell<Task>>>,
    /// The JavaScript's `clockLast`, which is the time the clock was read
    /// at the last time it was looked at.
    clock_last: f64,
    /// The JavaScript's `clockNow`, which is the time of the frame that is
    /// running.
    clock_now: f64,
    /// The JavaScript's `clockSkew`, which is what the clock of the program
    /// is read against.
    clock_skew: f64,
}

/// The JavaScript's timer, as it is in the list of the timers that are
/// waiting.
struct Task {
    /// The JavaScript's `_call`, which is the function that is called.
    call: Option<Rc<dyn Fn(f64)>>,
    /// The JavaScript's `_time`, which is the time the timer is due at.
    time: f64,
    /// The JavaScript's `_next`, which is the next timer of the list.
    next: Option<Rc<RefCell<Task>>>,
}

/// The state the timers and the list of timers share.
struct Shared {
    /// The JavaScript's module-level variables.
    state: RefCell<State>,
    /// The JavaScript's `clock`.
    clock: Box<dyn Clock>,
    /// The JavaScript's event loop, which is the browser's own.
    event_loop: RefCell<Box<dyn EventLoop>>,
}

/// The JavaScript's `restart`, which starts a timer again.
fn restart_task(
    shared: &Rc<Shared>,
    task: &Rc<RefCell<Task>>,
    call: Rc<dyn Fn(f64)>,
    delay: Option<f64>,
    time: Option<f64>,
) {
    let timers = Timers {
        shared: Rc::clone(shared),
    };
    // The JavaScript's `time = (time == null ? now() : +time) + (delay == null ? 0 : +delay);`
    let time = time.unwrap_or_else(|| timers.now()) + delay.unwrap_or(0.0);
    {
        let mut state = shared.state.borrow_mut();
        // The JavaScript's `if (!this._next && taskTail !== this)`, which
        // puts a timer that is not in the list at the end of it ✓.
        let linked = task.borrow().next.is_some();
        let is_tail = state
            .task_tail
            .as_ref()
            .map(|tail| Rc::ptr_eq(tail, task))
            .unwrap_or(false);
        if !linked && !is_tail {
            let tail = state.task_tail.clone();
            match tail {
                // The JavaScript's `if (taskTail) taskTail._next = this;`
                Some(tail) => tail.borrow_mut().next = Some(Rc::clone(task)),
                // The JavaScript's `else taskHead = this;`
                None => state.task_head = Some(Rc::clone(task)),
            }
            // The JavaScript's `taskTail = this;`
            state.task_tail = Some(Rc::clone(task));
        }
    }
    {
        // The JavaScript's `this._call = callback; this._time = time;`
        let mut task = task.borrow_mut();
        task.call = Some(call);
        task.time = time;
    }
    // The JavaScript's `sleep();`
    timers.sleep(None);
}

/// The JavaScript's `stop`, which stops a timer.
fn stop_task(shared: &Rc<Shared>, task: &Rc<RefCell<Task>>) {
    let stopped = {
        let mut task = task.borrow_mut();
        // The JavaScript's `if (this._call) {`
        if task.call.is_some() {
            // The JavaScript's `this._call = null; this._time = Infinity;`
            task.call = None;
            task.time = f64::INFINITY;
            true
        } else {
            false
        }
    };
    if stopped {
        // The JavaScript's `sleep();`
        Timers {
            shared: Rc::clone(shared),
        }
        .sleep(None);
    }
}

/// The JavaScript's `tick`, which is the function an interval calls again
/// and again.
///
/// The JavaScript's tick is a function that refers to itself, and the port's
/// is made afresh on every turn, which is the same thing without the
/// circle the two would make of each other ✓.
fn make_tick(
    shared: Weak<Shared>,
    task: Weak<RefCell<Task>>,
    total: Rc<Cell<f64>>,
    delay: f64,
    time: Option<f64>,
    callback: Rc<dyn Fn(f64)>,
) -> Rc<dyn Fn(f64)> {
    Rc::new(move |elapsed: f64| {
        let Some(shared) = shared.upgrade() else {
            return;
        };
        let Some(task) = task.upgrade() else {
            return;
        };
        // The JavaScript's `elapsed += total;`
        let elapsed = elapsed + total.get();
        // The JavaScript's `total += delay;`
        let new_total = total.get() + delay;
        total.set(new_total);
        // The JavaScript's `t.restart(tick, total += delay, time);`
        let tick = make_tick(
            Rc::downgrade(&shared),
            Rc::downgrade(&task),
            Rc::clone(&total),
            delay,
            time,
            Rc::clone(&callback),
        );
        restart_task(&shared, &task, tick, Some(new_total), time);
        // The JavaScript's `callback(elapsed);`
        callback(elapsed);
    })
}

/// The JavaScript's `clearNow`, which takes the cached time away.
fn clear_now_closure(shared: &Rc<Shared>) -> Box<dyn FnOnce()> {
    let shared = Rc::downgrade(shared);
    Box::new(move || {
        if let Some(shared) = shared.upgrade() {
            // The JavaScript's `clockNow = 0;`
            shared.state.borrow_mut().clock_now = 0.0;
        }
    })
}

/// The JavaScript's `wake`, as the loop of the program calls it.
fn wake_closure(shared: &Rc<Shared>) -> Box<dyn FnOnce()> {
    let shared = Rc::downgrade(shared);
    Box::new(move || {
        if let Some(shared) = shared.upgrade() {
            Timers { shared }.wake();
        }
    })
}

/// The JavaScript's `poke`, as the loop of the program calls it.
fn poke_closure(shared: &Rc<Shared>) -> Rc<dyn Fn()> {
    let shared = Rc::downgrade(shared);
    Rc::new(move || {
        if let Some(shared) = shared.upgrade() {
            Timers { shared }.poke();
        }
    })
}
