//! A port of **d3-dispatch**, from the JavaScript of the same name.
//!
//! Translated from `d3-dispatch/src/*.js`, version 1.0.6, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! # What the JavaScript is
//!
//! d3-dispatch is the small part of d3 that lets a program say "when this
//! happens, call this": the JavaScript's `dispatch` makes an object with a
//! list of *types*, `on` registers a *callback* under a type and a *name*,
//! and `call` and `apply` run the callbacks of a type.
//!
//! ```js
//! var d = dispatch("start", "end");
//! d.on("start", function(event) { console.log(event); });
//! d.call("start", this, {type: "start"});
//! ```
//!
//! # The shape of the port
//!
//! The JavaScript's callback is a function that is given the arguments and
//! the object the dispatch was called on; the port's is a closure with the
//! same two ✓, the second being a mutable reference to a value of any
//! type, so that a callback can read the object back as the type it knows
//! it to be.
//!
//! The JavaScript's arguments are of any type and are of any number; the
//! port's are of one type, which the dispatch carries, so that a callback
//! is given them without having to ask what they are.
//!
//! The JavaScript's `on` is one method that does three things, told apart
//! by how many arguments it was given: with one it gives the callback back,
//! with a callback it sets one, and with `null` it takes one away. Rust has
//! no such a method, so the port has three: [`Dispatch::on`],
//! [`Dispatch::on_get`] and [`Dispatch::on_null`].
//!
//! # The errors
//!
//! The JavaScript throws an `Error` where the port gives back a
//! [`DispatchError`], whose message is the JavaScript's own ✓.
//!
//! # What the port cannot do
//!
//! The port's `call` borrows the dispatch while the callbacks are running,
//! so a callback cannot change the dispatch it is called from. The
//! JavaScript's can, because its dispatch is an object that a callback may
//! have closed over, and the port has no way of handing it to one while it
//! is calling it.
//!
//! The JavaScript's `invalid callback` error cannot happen in the port,
//! because a closure is a closure ✓.

use std::any::Any;
use std::rc::Rc;

/// A callback of a dispatch, which the dispatch calls when its type is
/// called.
///
/// The JavaScript's callback is a function, which is called with the
/// arguments the dispatch was called with and with the object it was called
/// on; the port's is a closure with the same two ✓.
///
/// The callback is held behind an [`Rc`], because the JavaScript's `copy`
/// gives back a dispatch that calls the *same* functions as the one it was
/// copied from ✓.
pub type Callback<A> = Rc<dyn Fn(&mut dyn Any, &[A])>;

/// The JavaScript's `dispatch`, which makes a dispatch of the given types.
///
/// The JavaScript takes the types as its own arguments:
///
/// ```js
/// function dispatch() {
///   for (var i = 0, n = arguments.length, _ = {}, t; i < n; ++i) {
///     if (!(t = arguments[i] + "") || (t in _) || /[\s.]/.test(t)) throw new Error("illegal type: " + t);
///     _[t] = [];
///   }
///   return new Dispatch(_);
/// }
/// ```
///
/// The port takes them as a list of strings, and refuses the same ones:
/// a name that is empty, a name that is a type already made, and a name
/// with whitespace or a full stop in it ✓.
///
/// The JavaScript's map of the types is a plain object, and it inherits
/// from `Object.prototype`, so the names of the prototype's own properties
/// are refused as well — `dispatch("toString")` throws ✓. The port keeps
/// that, and [`OBJECT_PROTOTYPE_NAMES`] holds the names.
pub fn dispatch<A>(types: &[&str]) -> Result<Dispatch<A>, DispatchError> {
    let mut made: Vec<Type<A>> = Vec::new();
    for name in types {
        // The JavaScript's `!(t = arguments[i] + "")` refuses a name that
        // is empty, and the error carries the empty name.
        if name.is_empty() {
            return Err(DispatchError::IllegalType(String::new()));
        }
        // The JavaScript's `t in _` asks whether the name is a key of its
        // own map of the types, which is also a question about the
        // prototype the map inherits from.
        if made.iter().any(|t| t.name == *name) || OBJECT_PROTOTYPE_NAMES.contains(name) {
            return Err(DispatchError::IllegalType((*name).to_owned()));
        }
        // The JavaScript's `/[\s.]/.test(t)` refuses a name with
        // whitespace or a full stop in it.
        if name.chars().any(|c| is_whitespace(c) || c == '.') {
            return Err(DispatchError::IllegalType((*name).to_owned()));
        }
        made.push(Type {
            name: (*name).to_owned(),
            callbacks: Vec::new(),
        });
    }
    Ok(Dispatch { types: made })
}

/// The JavaScript's `Dispatch`, which holds the types and the callbacks
/// registered under them.
///
/// The JavaScript's own is made by [`dispatch`], and its methods are
/// `on`, `copy`, `call` and `apply` ✓.
pub struct Dispatch<A> {
    types: Vec<Type<A>>,
}

impl<A> Dispatch<A> {
    /// The JavaScript's `on`, which sets the callback of a type and a name.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// on: function(typename, callback) {
    ///   var _ = this._,
    ///       T = parseTypenames(typename + "", _),
    ///       t,
    ///       i = -1,
    ///       n = T.length;
    ///
    ///   if (arguments.length < 2) {
    ///     while (++i < n) if ((t = (typename = T[i]).type) && (t = get(_[t], typename.name))) return t;
    ///     return;
    ///   }
    ///
    ///   if (callback != null && typeof callback !== "function") throw new Error("invalid callback: " + callback);
    ///   while (++i < n) {
    ///     if (t = (typename = T[i]).type) _[t] = set(_[t], typename.name, callback);
    ///     else if (callback == null) for (t in _) _[t] = set(_[t], typename.name, null);
    ///   }
    ///
    ///   return this;
    /// }
    /// ```
    ///
    /// The JavaScript's callback is a function, and the port's is a closure
    /// ✓. The JavaScript's `invalid callback` error cannot happen in the
    /// port, because a closure is a closure ✓.
    ///
    /// A typename with no type in it — `".name"` — sets nothing, because
    /// the JavaScript only sets a callback when the typename has a type ✓.
    pub fn on<F>(&mut self, typename: &str, callback: F) -> Result<&mut Self, DispatchError>
    where
        F: Fn(&mut dyn Any, &[A]) + 'static,
    {
        let typenames = self.parse_typenames(typename)?;
        let callback: Callback<A> = Rc::new(callback);
        for t in &typenames {
            // The JavaScript's `if (t = (typename = T[i]).type)`, which
            // sets the callback of the type and the name. The JavaScript's
            // `else if (callback == null)` is the other half of this: a
            // typename with no type in it removes a callback, and a
            // typename with no type in it and a callback that is not null
            // does nothing at all ✓.
            if !t.type_name.is_empty() {
                self.set(&t.type_name, &t.name, Some(callback.clone()));
            }
        }
        Ok(self)
    }

    /// The JavaScript's `on` with no callback, which gives the callback of
    /// a type and a name back.
    ///
    /// The JavaScript's `if (arguments.length < 2)` reads the callback of
    /// the first typename that has one, and gives back nothing when there
    /// is none ✓.
    pub fn on_get(&self, typename: &str) -> Result<Option<Callback<A>>, DispatchError> {
        let typenames = self.parse_typenames(typename)?;
        for t in &typenames {
            // The JavaScript's `(t = (typename = T[i]).type) && (t =
            // get(_[t], typename.name))`, which asks for the type first and
            // then for the name.
            if !t.type_name.is_empty() {
                if let Some(callback) = self.get(&t.type_name, &t.name) {
                    return Ok(Some(callback));
                }
            }
        }
        Ok(None)
    }

    /// The JavaScript's `on` with a null callback, which takes the callback
    /// of a type and a name away.
    ///
    /// The JavaScript's `else if (callback == null) for (t in _) _[t] =
    /// set(_[t], typename.name, null)` removes the callback of that name
    /// from every type when the typename has no type in it ✓.
    pub fn on_null(&mut self, typename: &str) -> Result<&mut Self, DispatchError> {
        let typenames = self.parse_typenames(typename)?;
        for t in &typenames {
            if !t.type_name.is_empty() {
                self.set(&t.type_name, &t.name, None);
            } else {
                // The JavaScript's `for (t in _)`, which goes over every
                // type the dispatch has.
                for name in self.type_names() {
                    self.set(&name, &t.name, None);
                }
            }
        }
        Ok(self)
    }

    /// The JavaScript's `copy`, which copies the dispatch.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// copy: function() {
    ///   var copy = {}, _ = this._;
    ///   for (var t in _) copy[t] = _[t].slice();
    ///   return new Dispatch(copy);
    /// }
    /// ```
    ///
    /// The JavaScript's copy has the same functions in it as the one it was
    /// copied from — the array of the callbacks is copied, but the
    /// functions themselves are not — and the port's is the same ✓.
    pub fn copy(&self) -> Dispatch<A> {
        Dispatch {
            types: self
                .types
                .iter()
                .map(|t| Type {
                    name: t.name.clone(),
                    callbacks: t.callbacks.clone(),
                })
                .collect(),
        }
    }

    /// The JavaScript's `call`, which calls the callbacks of a type.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// call: function(type, that) {
    ///   if ((n = arguments.length - 2) > 0) for (var args = new Array(n), i = 0, n, t; i < n; ++i) args[i] = arguments[i + 2];
    ///   if (!this._.hasOwnProperty(type)) throw new Error("unknown type: " + type);
    ///   for (t = this._[type], i = 0, n = t.length; i < n; ++i) t[i].value.apply(that, args);
    /// }
    /// ```
    ///
    /// The JavaScript's `call` takes the arguments one after another and
    /// its `apply` takes them as a list; in Rust a list is a list, so the
    /// port's two methods are the same ✓.
    ///
    /// The JavaScript's `this._.hasOwnProperty(type)` asks whether the
    /// dispatch has the type at all, and the names of `Object.prototype`
    /// are not among them ✓.
    ///
    /// The JavaScript's `type` is a word in Rust, so the port calls it
    /// `type_name`.
    pub fn call(
        &self,
        type_name: &str,
        that: &mut dyn Any,
        arguments: &[A],
    ) -> Result<(), DispatchError> {
        let t = self
            .type_at(type_name)
            .ok_or_else(|| DispatchError::UnknownType(type_name.to_owned()))?;
        // The JavaScript's `for (t = this._[type], i = 0, n = t.length; i <
        // n; ++i) t[i].value.apply(that, args)`, which takes the count of
        // the callbacks before the first one is called.
        for registration in &t.callbacks {
            (registration.callback)(that, arguments);
        }
        Ok(())
    }

    /// The JavaScript's `apply`, which calls the callbacks of a type with a
    /// list of arguments.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// apply: function(type, that, args) {
    ///   if (!this._.hasOwnProperty(type)) throw new Error("unknown type: " + type);
    ///   for (var t = this._[type], i = 0, n = t.length; i < n; ++i) t[i].value.apply(that, args);
    /// }
    /// ```
    pub fn apply(
        &self,
        type_name: &str,
        that: &mut dyn Any,
        arguments: &[A],
    ) -> Result<(), DispatchError> {
        self.call(type_name, that, arguments)
    }

    /// The JavaScript's `parseTypenames`, which takes a typename apart into
    /// its types and their names.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function parseTypenames(typenames, types) {
    ///   return typenames.trim().split(/^|\s+/).map(function(t) {
    ///     var name = "", i = t.indexOf(".");
    ///     if (i >= 0) name = t.slice(i + 1), t = t.slice(0, i);
    ///     if (t && !types.hasOwnProperty(t)) throw new Error("unknown type: " + t);
    ///     return {type: t, name: name};
    ///   });
    /// }
    /// ```
    ///
    /// The JavaScript's `types.hasOwnProperty(t)` asks whether the type is
    /// one of the dispatch's own ✓.
    fn parse_typenames(&self, typenames: &str) -> Result<Vec<Typename>, DispatchError> {
        let trimmed = trim(typenames);
        let mut made = Vec::new();
        for token in split(&trimmed) {
            // The JavaScript's `i = t.indexOf(".")`, which finds the first
            // full stop, and takes the name to be what is after it.
            let (type_name, name) = match token.find('.') {
                Some(at) => (&token[..at], &token[at + 1..]),
                None => (token, ""),
            };
            if !type_name.is_empty() && !self.has_type(type_name) {
                return Err(DispatchError::UnknownType(type_name.to_owned()));
            }
            made.push(Typename {
                type_name: type_name.to_owned(),
                name: name.to_owned(),
            });
        }
        Ok(made)
    }

    /// The JavaScript's `set`, which sets the callback of a name, or takes
    /// it away.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function set(type, name, callback) {
    ///   for (var i = 0, n = type.length; i < n; ++i) {
    ///     if (type[i].name === name) {
    ///       type[i] = noop, type = type.slice(0, i).concat(type.slice(i + 1));
    ///       break;
    ///     }
    ///   }
    ///   if (callback != null) type.push({name: name, value: callback});
    ///   return type;
    /// }
    /// ```
    ///
    /// The JavaScript takes the old callback out and puts the new one at
    /// the end, so a callback that is set again is called after the ones
    /// that were set before it ✓.
    ///
    /// The JavaScript's `type[i] = noop` leaves a callback that does
    /// nothing in the array it is taking the old callback out of, which
    /// matters only when a callback takes itself away while it is being
    /// called. The port's callbacks cannot do that, so the entry is taken
    /// out of the list.
    fn set(&mut self, type_name: &str, name: &str, callback: Option<Callback<A>>) {
        let t = match self.type_at_mut(type_name) {
            Some(t) => t,
            None => return,
        };
        if let Some(at) = t.callbacks.iter().position(|r| r.name == name) {
            t.callbacks.remove(at);
        }
        if let Some(callback) = callback {
            t.callbacks.push(Registration {
                name: name.to_owned(),
                callback,
            });
        }
    }

    /// The JavaScript's `get`, which gives the callback of a name back.
    ///
    /// The JavaScript:
    ///
    /// ```js
    /// function get(type, name) {
    ///   for (var i = 0, n = type.length, c; i < n; ++i) {
    ///     if ((c = type[i]).name === name) {
    ///       return c.value;
    ///     }
    ///   }
    /// }
    /// ```
    fn get(&self, type_name: &str, name: &str) -> Option<Callback<A>> {
        let t = self.type_at(type_name)?;
        t.callbacks
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.callback.clone())
    }

    /// Whether the dispatch has the type.
    fn has_type(&self, type_name: &str) -> bool {
        self.types.iter().any(|t| t.name == type_name)
    }

    /// The type of the dispatch, if it has it.
    fn type_at(&self, type_name: &str) -> Option<&Type<A>> {
        self.types.iter().find(|t| t.name == type_name)
    }

    /// The type of the dispatch, if it has it, to be changed.
    fn type_at_mut(&mut self, type_name: &str) -> Option<&mut Type<A>> {
        self.types.iter_mut().find(|t| t.name == type_name)
    }

    /// The names of the types, in the order they were made.
    fn type_names(&self) -> Vec<String> {
        self.types.iter().map(|t| t.name.clone()).collect()
    }
}

impl<A> std::fmt::Debug for Dispatch<A> {
    /// The types of the dispatch, with the count of the callbacks of each.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let types: Vec<(&str, usize)> = self
            .types
            .iter()
            .map(|t| (t.name.as_str(), t.callbacks.len()))
            .collect();
        f.debug_struct("Dispatch").field("types", &types).finish()
    }
}

/// A type of a dispatch, with the callbacks registered under it.
struct Type<A> {
    name: String,
    callbacks: Vec<Registration<A>>,
}

/// A callback of a dispatch, registered under a name.
struct Registration<A> {
    name: String,
    callback: Callback<A>,
}

impl<A> Clone for Registration<A> {
    /// The JavaScript's copy of the array of the callbacks keeps the same
    /// functions in it, so the port's copy of a registration keeps the same
    /// callback ✓.
    fn clone(&self) -> Self {
        Registration {
            name: self.name.clone(),
            callback: self.callback.clone(),
        }
    }
}

/// A typename of the JavaScript, which is a type and a name.
struct Typename {
    type_name: String,
    name: String,
}

/// What can go wrong while a dispatch is used.
///
/// The JavaScript throws an `Error` with a message for each of these, and
/// the port's messages are the JavaScript's own ✓.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchError {
    /// The JavaScript throws `"illegal type: " + t` for a type that is
    /// empty, that is made already, or that has a whitespace or a full
    /// stop in it.
    IllegalType(String),
    /// The JavaScript throws `"unknown type: " + t` for a type the
    /// dispatch does not have.
    UnknownType(String),
    /// The JavaScript throws `"invalid callback: " + callback` for a
    /// callback that is not a function.
    ///
    /// The port's callbacks are closures, so this cannot happen in the
    /// port ✓.
    InvalidCallback(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::IllegalType(t) => write!(f, "illegal type: {t}"),
            DispatchError::UnknownType(t) => write!(f, "unknown type: {t}"),
            DispatchError::InvalidCallback(callback) => write!(f, "invalid callback: {callback}"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// The names of the properties of the JavaScript's `Object.prototype`.
///
/// The JavaScript's map of the types is a plain object, and it inherits
/// from `Object.prototype`, so the JavaScript's `t in _` is true for these
/// names, and `dispatch("toString")` throws ✓. The port keeps that, and
/// the list is checked against the JavaScript's own ✓.
pub const OBJECT_PROTOTYPE_NAMES: [&str; 12] = [
    "constructor",
    "__defineGetter__",
    "__defineSetter__",
    "hasOwnProperty",
    "__lookupGetter__",
    "__lookupSetter__",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toString",
    "valueOf",
    "__proto__",
    "toLocaleString",
];

/// Whether the character is whitespace in the JavaScript's sense.
///
/// The JavaScript's `\s` is the same as the whitespace its `trim` takes
/// off, and it is not quite Rust's `char::is_whitespace`: the JavaScript
/// has the zero width no-break space and Rust does not, and Rust has the
/// next line and the JavaScript does not ✓.
fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000a}'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// The JavaScript's `trim`, which takes the whitespace off both ends.
fn trim(text: &str) -> String {
    text.trim_matches(is_whitespace).to_owned()
}

/// The JavaScript's split of a typename on `/^|\s+/`.
///
/// The JavaScript's split gives nothing for an empty string, and the
/// separator is a run of whitespace, so the port's is the same ✓.
fn split(text: &str) -> Vec<&str> {
    text.split(is_whitespace)
        .filter(|s| !s.is_empty())
        .collect()
}
