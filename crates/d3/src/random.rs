//! A port of **d3-random**, from the JavaScript of the same name.
//!
//! Translated from `d3-random/src/*.js`, version 1.1.2, of d3 5.16.0. The
//! JavaScript is the authority; where the two could be read differently, the
//! JavaScript is right.
//!
//! d3-random is the set of generators of random numbers. Each of them is
//! made from a *source* — a function that gives a number in `[0, 1)` — and
//! each of them gives back a *generator*, which is a function that gives a
//! number of its own kind.
//!
//! # The shape of the port
//!
//! The JavaScript's generators are functions, and the port's are structs
//! with a `next` method: a generator that keeps a number between its calls
//! — the normal one keeps the first of a pair of numbers — cannot be a
//! closure in Rust without a borrow of itself. The generators are iterators
//! as well, so that a program can take numbers out of them with the
//! standard library's own ways ✓.
//!
//! The JavaScript hangs the source on the function itself, as
//! `randomUniform.source(mySource)`, and the port has a function of its own
//! for each export: `random_uniform_source` ✓.
//!
//! # The default source
//!
//! The JavaScript's default source is `Math.random`, which the
//! specification does not give: it is a number in `[0, 1)`, and the stream
//! behind it is the engine's own. The port's default source is a generator
//! of its own, seeded from the clock, and it gives numbers in `[0, 1)` ✓.
//! The JavaScript's stream is one that every generator draws from, and so is
//! the port's — one per thread ✓, for a thread is the nearest thing a Rust
//! program has to the realm the JavaScript's stream belongs to.

// The JavaScript's generator is a function that gives a number, and the
// port's `next` is that function: it gives the number itself and not an
// `Option` of it, for the generator does not end. The lint would have the
// method take the shape of `Iterator::next`, which is the standard
// library's own and not the JavaScript's.
#![allow(clippy::should_implement_trait)]

use crate::ieee754;
use crate::js;

use std::cell::RefCell;

/// A source of random numbers in `[0, 1)`, which is what the JavaScript's
/// `source` is.
///
/// The JavaScript's source is a function that gives such a number, and the
/// port's is a closure with the same shape ✓.
///
/// The JavaScript's source is one function that several generators may
/// draw from, and the port's is owned by the generator that was given it.
/// A caller who wants two generators to share one gives them a source that
/// shares it, with a `RefCell` or the like ✓.
pub type Source = Box<dyn FnMut() -> f64>;

/// d3-random's `defaultSource`, which is the JavaScript's `Math.random`.
///
/// The JavaScript:
///
/// ```js
/// export default function() {
///   return Math.random();
/// }
/// ```
///
/// The specification does not say what `Math.random` gives beyond that it
/// is a number in `[0, 1)`, and the stream behind it is the engine's own.
/// The port's default source is a generator of its own, seeded from the
/// clock, and it gives numbers in `[0, 1)` ✓.
///
/// Every generator that is made without a source of its own draws from the
/// same stream, as the JavaScript's do ✓.
pub fn default_source() -> Source {
    Box::new(|| THREAD_SOURCE.with(|source| source.borrow_mut().next()))
}

/// The port's own generator, a linear congruential generator of the kind
/// the Numerical Recipes gives.
struct Lcg {
    state: u32,
}

impl Lcg {
    /// A generator with a seed.
    fn new(seed: u32) -> Self {
        Lcg { state: seed }
    }

    /// The next number, in `[0, 1)`.
    ///
    /// The state is taken modulo two to the thirty-two and the number is
    /// the state over two to the thirty-two, which is exact: a double holds
    /// the state's thirty-two bits, and the division is by a power of two.
    fn next(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        f64::from(self.state) / 4_294_967_296.0
    }
}

thread_local! {
    // The thread's own stream, which every generator that is given no
    // source of its own draws from.
    static THREAD_SOURCE: RefCell<Lcg> = RefCell::new(Lcg::new(seed()));
}

/// The seed of the thread's stream, taken from the clock.
///
/// The clock's own nanoseconds are not much of a seed, so the place the
/// seed was taken at is mixed into it as well.
fn seed() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.subsec_nanos())
        .unwrap_or(0);
    let here = &nanos as *const u32 as usize as u32;
    nanos ^ here.rotate_left(13) ^ 0x9e37_79b9
}

/// A generator of uniform numbers, which is what the JavaScript's
/// `randomUniform` gives back.
pub struct Uniform {
    source: Source,
    min: f64,
    max: f64,
}

impl Uniform {
    /// The next number.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// return function() {
    ///   return source() * max + min;
    /// };
    /// ```
    pub fn next(&mut self) -> f64 {
        self.draw()
    }

    /// The number the generator draws.
    fn draw(&mut self) -> f64 {
        (self.source)() * self.max + self.min
    }
}

impl Iterator for Uniform {
    type Item = f64;

    /// The next number, which is always there: the JavaScript's generator
    /// does not end ✓.
    fn next(&mut self) -> Option<f64> {
        Some(self.draw())
    }
}

/// The ends of a uniform generator's numbers, from the JavaScript's
/// arguments.
///
/// The JavaScript:
///
/// ```js
/// min = min == null ? 0 : +min;
/// max = max == null ? 1 : +max;
/// if (arguments.length === 1) max = min, min = 0;
/// else max -= min;
/// ```
///
/// The port's `random_uniform` cannot tell one argument from two, for a
/// Rust call has one shape, so it follows the rule that a `min` that is
/// there with a `max` that is not is the JavaScript's one argument ✓, whose
/// bound is the `min` ✓. The JavaScript's own spellings are kept by
/// [`random_uniform_args`] ✓.
fn uniform_bounds(min: Option<f64>, max: Option<f64>) -> (f64, f64) {
    match (min, max) {
        // The JavaScript's one argument: it is the upper bound, and the
        // numbers are from zero to it.
        (Some(min), None) => (0.0, js::number(min)),
        // The JavaScript's two arguments, or none at all: the numbers are
        // from the first to the second.
        (min, max) => {
            let min = min.map(js::number).unwrap_or(0.0);
            let max = max.map(js::number).unwrap_or(1.0);
            (min, max - min)
        }
    }
}

/// d3-random's `randomUniform`, the generator of uniform numbers.
///
/// The JavaScript:
///
/// ```js
/// export default (function sourceRandomUniform(source) {
///   function randomUniform(min, max) {
///     min = min == null ? 0 : +min;
///     max = max == null ? 1 : +max;
///     if (arguments.length === 1) max = min, min = 0;
///     else max -= min;
///     return function() {
///       return source() * max + min;
///     };
///   }
///
///   randomUniform.source = sourceRandomUniform;
///
///   return randomUniform;
/// })(defaultSource);
/// ```
///
/// The JavaScript reads how many arguments it was given, and the port's
/// `random_uniform` cannot, for a Rust call has one shape. The rule the
/// port follows is that a `min` that is there with a `max` that is not is
/// the JavaScript's one argument, whose bound is the `min` ✓, and that no
/// arguments at all is the numbers in `[0, 1)` ✓.
///
/// The JavaScript's own spellings — an argument that was `null`, and an
/// argument that was not there at all — are kept by [`random_uniform_args`]
/// ✓.
pub fn random_uniform(min: Option<f64>, max: Option<f64>) -> Uniform {
    random_uniform_source(default_source(), min, max)
}

/// d3-random's `randomUniform`, with the arguments the JavaScript's
/// `arguments` object makes possible.
///
/// The JavaScript's `randomUniform` reads how many arguments it was given,
/// and the port's `random_uniform` cannot. This is that shape: the list is
/// the arguments the JavaScript was given, and a `None` in it is an
/// argument that was `null` or one that was not there at all ✓.
///
/// The JavaScript's one argument is the upper bound of the numbers ✓, and
/// its two are the ends of them ✓.
pub fn random_uniform_args(args: &[Option<f64>]) -> Uniform {
    random_uniform_source_args(default_source(), args)
}

/// d3-random's `randomUniform.source`, the generator of uniform numbers
/// from a source of one's own.
///
/// The JavaScript's `randomUniform.source` is the `sourceRandomUniform` of
/// the export above ✓.
pub fn random_uniform_source(source: Source, min: Option<f64>, max: Option<f64>) -> Uniform {
    let (min, max) = uniform_bounds(min, max);
    Uniform { source, min, max }
}

/// d3-random's `randomUniform.source`, with the arguments the JavaScript's
/// `arguments` object makes possible.
pub fn random_uniform_source_args(source: Source, args: &[Option<f64>]) -> Uniform {
    let (min, max) = match args {
        // The JavaScript's no arguments at all.
        [] => (0.0, 1.0),
        // The JavaScript's one argument, which is the upper bound of the
        // numbers, and the JavaScript's `max = min, min = 0` ✓.
        [min] => (0.0, min.map(js::number).unwrap_or(0.0)),
        // The JavaScript's two arguments, or more, of which it reads two.
        [min, max, ..] => {
            let min = min.map(js::number).unwrap_or(0.0);
            let max = max.map(js::number).unwrap_or(1.0);
            (min, max - min)
        }
    };
    Uniform { source, min, max }
}

/// A generator of normal numbers, which is what the JavaScript's
/// `randomNormal` gives back.
pub struct Normal {
    source: Source,
    mu: f64,
    sigma: f64,
    /// The first of the last pair of numbers the generator drew, which is
    /// kept for the next number ✓.
    x: Option<f64>,
    /// The square of that pair, which is kept with it ✓.
    r: f64,
}

impl Normal {
    /// The next number.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// return function() {
    ///   var y;
    ///
    ///   // If available, use the second previously-generated uniform random.
    ///   if (x != null) y = x, x = null;
    ///
    ///   // Otherwise, generate a new x and y.
    ///   else do {
    ///     x = source() * 2 - 1;
    ///     y = source() * 2 - 1;
    ///     r = x * x + y * y;
    ///   } while (!r || r > 1);
    ///
    ///   return mu + sigma * y * Math.sqrt(-2 * Math.log(r) / r);
    /// };
    /// ```
    pub fn next(&mut self) -> f64 {
        self.draw()
    }

    /// The number the generator draws.
    fn draw(&mut self) -> f64 {
        let y = match self.x.take() {
            // The JavaScript's `if (x != null) y = x, x = null;`, which
            // keeps the square of the pair and uses the pair's first number.
            Some(x) => x,
            // The JavaScript's own do-while, which throws the pair away
            // when the circle the pair is in is not the unit one.
            None => loop {
                let x = (self.source)() * 2.0 - 1.0;
                let y = (self.source)() * 2.0 - 1.0;
                let r = x * x + y * y;
                // The JavaScript's `while (!r || r > 1)`, whose `!r` is
                // true of a zero and of a number that is not a number ✓.
                if !(r == 0.0 || r.is_nan() || r > 1.0) {
                    self.x = Some(x);
                    self.r = r;
                    break y;
                }
            },
        };
        self.mu + self.sigma * y * ieee754::sqrt(-2.0 * ieee754::log(self.r) / self.r)
    }
}

impl Iterator for Normal {
    type Item = f64;

    /// The next number, which is always there: the JavaScript's generator
    /// does not end ✓.
    fn next(&mut self) -> Option<f64> {
        Some(self.draw())
    }
}

/// d3-random's `randomNormal`, the generator of normal numbers.
///
/// The JavaScript:
///
/// ```js
/// export default (function sourceRandomNormal(source) {
///   function randomNormal(mu, sigma) {
///     var x, r;
///     mu = mu == null ? 0 : +mu;
///     sigma = sigma == null ? 1 : +sigma;
///     return function() {
///       ...
///     };
///   }
///
///   randomNormal.source = sourceRandomNormal;
///
///   return randomNormal;
/// })(defaultSource);
/// ```
///
/// The JavaScript's `mu` is the middle of the numbers and its `sigma` is
/// how far they spread, and neither of them is required: without them the
/// numbers are the ordinary ones ✓.
pub fn random_normal(mu: Option<f64>, sigma: Option<f64>) -> Normal {
    random_normal_source(default_source(), mu, sigma)
}

/// d3-random's `randomNormal.source`, the generator of normal numbers from
/// a source of one's own.
pub fn random_normal_source(source: Source, mu: Option<f64>, sigma: Option<f64>) -> Normal {
    Normal {
        source,
        mu: mu.map(js::number).unwrap_or(0.0),
        sigma: sigma.map(js::number).unwrap_or(1.0),
        x: None,
        r: 0.0,
    }
}

/// A generator of log-normal numbers, which is what the JavaScript's
/// `randomLogNormal` gives back.
pub struct LogNormal {
    normal: Normal,
}

impl LogNormal {
    /// The next number.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// return function() {
    ///   return Math.exp(randomNormal());
    /// };
    /// ```
    pub fn next(&mut self) -> f64 {
        self.draw()
    }

    /// The number the generator draws.
    fn draw(&mut self) -> f64 {
        ieee754::exp(self.normal.next())
    }
}

impl Iterator for LogNormal {
    type Item = f64;

    /// The next number, which is always there: the JavaScript's generator
    /// does not end ✓.
    fn next(&mut self) -> Option<f64> {
        Some(self.draw())
    }
}

/// d3-random's `randomLogNormal`, the generator of log-normal numbers.
///
/// The JavaScript:
///
/// ```js
/// export default (function sourceRandomLogNormal(source) {
///   function randomLogNormal() {
///     var randomNormal = normal.source(source).apply(this, arguments);
///     return function() {
///       return Math.exp(randomNormal());
///     };
///   }
///
///   randomLogNormal.source = sourceRandomLogNormal;
///
///   return randomLogNormal;
/// })(defaultSource);
/// ```
///
/// The JavaScript's `randomLogNormal` takes no arguments of its own, and
/// gives the arguments it was given to the normal generator ✓, which is
/// what the port's `mu` and `sigma` are ✓.
pub fn random_log_normal(mu: Option<f64>, sigma: Option<f64>) -> LogNormal {
    random_log_normal_source(default_source(), mu, sigma)
}

/// d3-random's `randomLogNormal.source`, the generator of log-normal
/// numbers from a source of one's own.
pub fn random_log_normal_source(source: Source, mu: Option<f64>, sigma: Option<f64>) -> LogNormal {
    LogNormal {
        normal: random_normal_source(source, mu, sigma),
    }
}

/// A generator of exponential numbers, which is what the JavaScript's
/// `randomExponential` gives back.
pub struct Exponential {
    source: Source,
    lambda: f64,
}

impl Exponential {
    /// The next number.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// return function() {
    ///   return -Math.log(1 - source()) / lambda;
    /// };
    /// ```
    pub fn next(&mut self) -> f64 {
        self.draw()
    }

    /// The number the generator draws.
    fn draw(&mut self) -> f64 {
        -ieee754::log(1.0 - (self.source)()) / self.lambda
    }
}

impl Iterator for Exponential {
    type Item = f64;

    /// The next number, which is always there: the JavaScript's generator
    /// does not end ✓.
    fn next(&mut self) -> Option<f64> {
        Some(self.draw())
    }
}

/// d3-random's `randomExponential`, the generator of exponential numbers.
///
/// The JavaScript:
///
/// ```js
/// export default (function sourceRandomExponential(source) {
///   function randomExponential(lambda) {
///     return function() {
///       return -Math.log(1 - source()) / lambda;
///     };
///   }
///
///   randomExponential.source = sourceRandomExponential;
///
///   return randomExponential;
/// })(defaultSource);
/// ```
///
/// The JavaScript's `lambda` is used as it is, so a `lambda` that was not
/// there at all is a division by nothing, which is a number that is not a
/// number ✓, and the JavaScript's `null` is a division by zero, which the
/// port's caller writes as `Some(0.0)` ✓.
pub fn random_exponential(lambda: Option<f64>) -> Exponential {
    random_exponential_source(default_source(), lambda)
}

/// d3-random's `randomExponential.source`, the generator of exponential
/// numbers from a source of one's own.
pub fn random_exponential_source(source: Source, lambda: Option<f64>) -> Exponential {
    Exponential {
        source,
        lambda: lambda.map(js::number).unwrap_or(f64::NAN),
    }
}

/// A generator of Bates numbers, which is what the JavaScript's
/// `randomBates` gives back.
pub struct Bates {
    irwin_hall: IrwinHall,
    n: f64,
}

impl Bates {
    /// The next number.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// return function() {
    ///   return randomIrwinHall() / n;
    /// };
    /// ```
    pub fn next(&mut self) -> f64 {
        self.draw()
    }

    /// The number the generator draws.
    fn draw(&mut self) -> f64 {
        self.irwin_hall.next() / self.n
    }
}

impl Iterator for Bates {
    type Item = f64;

    /// The next number, which is always there: the JavaScript's generator
    /// does not end ✓.
    fn next(&mut self) -> Option<f64> {
        Some(self.draw())
    }
}

/// d3-random's `randomBates`, the generator of Bates numbers.
///
/// The JavaScript:
///
/// ```js
/// export default (function sourceRandomBates(source) {
///   function randomBates(n) {
///     var randomIrwinHall = irwinHall.source(source)(n);
///     return function() {
///       return randomIrwinHall() / n;
///     };
///   }
///
///   randomBates.source = sourceRandomBates;
///
///   return randomBates;
/// })(defaultSource);
/// ```
///
/// The JavaScript's `n` is the number of the uniform numbers the sum is
/// made of, and the sum is divided by it ✓. A `n` that was not there at
/// all is a division by nothing ✓.
pub fn random_bates(n: Option<f64>) -> Bates {
    random_bates_source(default_source(), n)
}

/// d3-random's `randomBates.source`, the generator of Bates numbers from a
/// source of one's own.
pub fn random_bates_source(source: Source, n: Option<f64>) -> Bates {
    let n = n.map(js::number).unwrap_or(f64::NAN);
    Bates {
        irwin_hall: IrwinHall { source, n },
        n,
    }
}

/// A generator of Irwin-Hall numbers, which is what the JavaScript's
/// `randomIrwinHall` gives back.
pub struct IrwinHall {
    source: Source,
    n: f64,
}

impl IrwinHall {
    /// The next number.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// return function() {
    ///   for (var sum = 0, i = 0; i < n; ++i) sum += source();
    ///   return sum;
    /// };
    /// ```
    pub fn next(&mut self) -> f64 {
        self.draw()
    }

    /// The number the generator draws.
    fn draw(&mut self) -> f64 {
        let mut sum = 0.0;
        // The JavaScript's `i` is a number and is compared with the `n` as
        // one ✓, so a `n` of two and a half is three numbers ✓.
        let mut i = 0.0;
        while i < self.n {
            sum += (self.source)();
            i += 1.0;
        }
        sum
    }
}

impl Iterator for IrwinHall {
    type Item = f64;

    /// The next number, which is always there: the JavaScript's generator
    /// does not end ✓.
    fn next(&mut self) -> Option<f64> {
        Some(self.draw())
    }
}

/// d3-random's `randomIrwinHall`, the generator of Irwin-Hall numbers.
///
/// The JavaScript:
///
/// ```js
/// export default (function sourceRandomIrwinHall(source) {
///   function randomIrwinHall(n) {
///     return function() {
///       for (var sum = 0, i = 0; i < n; ++i) sum += source();
///       return sum;
///     };
///   }
///
///   randomIrwinHall.source = sourceRandomIrwinHall;
///
///   return randomIrwinHall;
/// })(defaultSource);
/// ```
///
/// The JavaScript's `n` is the number of the uniform numbers the sum is
/// made of ✓, and it is used as it is, so a `n` that was not there at all
/// is no numbers at all ✓.
pub fn random_irwin_hall(n: Option<f64>) -> IrwinHall {
    random_irwin_hall_source(default_source(), n)
}

/// d3-random's `randomIrwinHall.source`, the generator of Irwin-Hall
/// numbers from a source of one's own.
pub fn random_irwin_hall_source(source: Source, n: Option<f64>) -> IrwinHall {
    IrwinHall {
        source,
        n: n.map(js::number).unwrap_or(f64::NAN),
    }
}
