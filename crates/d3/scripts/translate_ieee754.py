#!/usr/bin/env python3
"""Translate V8's ieee754.cc into Rust.

The JavaScript this crate is a port of runs on V8, and V8's `Math.cos`,
`Math.sin` and the rest are its own copy of fdlibm rather than the
platform's. The two differ: on this machine the platform's `cos` differs
from V8's on about one argument in twenty-five, by one unit in the last
place, and a unit in the last place is enough to change the last digit of
a path.

So the port carries its own copy of those functions, and this script makes
that copy from V8's C++ rather than from anyone's memory of it.

The translation is mechanical. The C++ here is fdlibm of Sun Microsystems
with V8's changes, and it is straight arithmetic on doubles with a few bit
tricks. What has to change is:

  - the macros that take a double apart into two 32-bit words and put it
    back together, which become functions on the crate's own;
  - the C's types, which become Rust's;
  - the C's casts, which are the same as Rust's for these types;
  - the C's `0x1.0p-1` hexadecimal floats, which Rust writes as a
    multiplication by a power of two;
  - the `static const double` tables, which become Rust's arrays and
    constants;
  - the C's own `double` literals, which are kept exactly as they are
    written, because the last digit of a literal is part of the number.

Nothing is rewritten by hand, so the port's arithmetic is the C's
arithmetic, and the two agree to the last bit.
"""

import re
import sys
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / "ieee754.cc"
TARGET = HERE.parent / "src" / "ieee754.rs"

HEADER = '''//! The transcendental functions, as the JavaScript has them.
//!
//! Translated from V8's `src/base/ieee754.cc`, version 12.0.267.10, which
//! is itself adapted from fdlibm by Sun Microsystems.
//!
//! # Why the port carries its own
//!
//! The JavaScript this crate is a port of runs on V8, and V8's `Math.cos`,
//! `Math.sin`, `Math.exp` and the rest are V8's own copy of fdlibm rather
//! than the platform's. The two differ: on this machine the platform's
//! `cos` differs from V8's on about one argument in twenty-five, by one
//! unit in the last place, and a unit in the last place is enough to
//! change the last digit of a path.
//!
//! So the port carries its own copy, translated from V8's C++ by
//! `scripts/translate_ieee754.py`, and the two agree to the last bit.
//!
//! # The licence
//!
//! fdlibm's licence, and V8's changes to it, are BSD-style, and they are
//! in `LICENSE.ieee754`.

// The C's own style is kept, for the port is a translation of the C and
// not a rewrite of it: the names are the C's, the brackets are the C's,
// the semicolons are the C's, and the parts the C does not use are the
// C's too.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(redundant_semicolons)]
#![allow(unused_assignments)]
#![allow(unused_imports)]
#![allow(unused_labels)]
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::unused_unit)]
#![allow(clippy::let_and_return)]
#![allow(clippy::needless_return)]
#![allow(clippy::assign_op_pattern)]

#![allow(clippy::approx_constant)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::type_complexity)]
#![allow(clippy::double_parens)]
#![allow(clippy::eq_op)]
#![allow(clippy::excessive_precision)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::num::Wrapping;

type Int32 = i32;
type Uint32 = u32;
type Int64 = i64;
type Uint64 = u64;

/// A double, from its bits.
#[inline]
fn from_bits(bits: u64) -> f64 {
    f64::from_bits(bits)
}

/// Take a double apart into its two 32-bit words, the high one first.
#[inline]
fn extract_words(d: f64) -> (u32, u32) {
    let bits = d.to_bits();
    ((bits >> 32) as u32, bits as u32)
}

/// The more significant 32 bits of a double.
#[inline]
fn get_high_word(d: f64) -> u32 {
    (d.to_bits() >> 32) as u32
}

/// The less significant 32 bits of a double.
#[inline]
fn get_low_word(d: f64) -> u32 {
    d.to_bits() as u32
}

/// Put a double together from its two 32-bit words.
#[inline]
fn insert_words(ix0: u32, ix1: u32) -> f64 {
    f64::from_bits(((ix0 as u64) << 32) | (ix1 as u64))
}

/// Set the more significant 32 bits of a double.
#[inline]
fn set_high_word(d: &mut f64, v: u32) {
    let mut bits = d.to_bits();
    bits &= 0x0000_0000_FFFF_FFFF;
    bits |= (v as u64) << 32;
    *d = f64::from_bits(bits);
}

/// The square root, which is the same in every JavaScript and in Rust,
/// for it is the platform's own and it is exact.
#[inline]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// The C++'s `NegateWithWraparound`, which is a negation that does not
/// overflow: the smallest number is its own negation.
#[inline]
fn negate_with_wraparound(a: i32) -> i32 {
    if a == i32::MIN {
        a
    } else {
        -a
    }
}

/// The C++'s `SubWithWraparound`, which is a subtraction that does not
/// overflow.
#[inline]
fn sub_with_wraparound(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

/// Set the less significant 32 bits of a double.
#[inline]
fn set_low_word(d: &mut f64, v: u32) {
    let mut bits = d.to_bits();
    bits &= 0xFFFF_FFFF_0000_0000;
    bits |= v as u64;
    *d = f64::from_bits(bits);
}
'''


def strip_comments(text):
    """Take out the C comments and its preprocessor lines, keeping the code."""
    original = text
    # The C's `#define`s, which are the port's own constants and
    # aliases. The ones that are a function-like macro are the ones that
    # take a double apart and put it together, and they are handled
    # elsewhere.
    defines = []
    out = []
    keeping = False
    for line_number, line in enumerate(text.splitlines(keepends=True)):
        stripped = line.strip()
        if stripped.startswith("#") and not stripped.startswith("#define"):
            # The `#undef` and the `#endif`, which are the C's own
            # bookkeeping and not code.
            continue
        if stripped.startswith("#define"):
            # A macro that is a name and a value, which the port writes
            # out as a constant.
            simple = re.match(
                r"#define\s+(\w+)(?:\([^)]*\))?\s+(\S+)\s*$", stripped)
            if simple and simple.group(1) not in (
                    "EXTRACT_WORDS", "GET_HIGH_WORD", "GET_LOW_WORD",
                    "INSERT_WORDS", "SET_HIGH_WORD", "SET_LOW_WORD"):
                name, value = simple.group(1), simple.group(2)
                # An entry of a table, which the port writes out as a
                # constant of its own.
                entry = re.match(r"^(\w+)\[(\d+)\]$", value)
                if entry:
                    defines.append((name, ("table", entry.group(1),
                                           int(entry.group(2))), line_number))
                elif re.match(r"^\w+$", value):
                    defines.append((name, ("alias", value), line_number))
                else:
                    defines.append((name, value, line_number))
                continue
            # The macro's own definition, which is kept until the block
            # it opens is closed, and the block is taken out whole.
            keeping = stripped.endswith("\\") or "do {" in stripped
            continue
        if keeping:
            if "while (false)" in stripped:
                keeping = False
            continue
        out.append(line)
    text = "".join(out)
    # The place each `#define` was written in, which is the function it
    # is inside. The text as it was is used, because the preprocessor's
    # lines are no longer in it.
    # The place of each line, so that a `#define`'s may be found.
    places = []
    place = 0
    for line in original.splitlines(keepends=True):
        places.append(place)
        place += len(line)
    for name, value, line_number in defines:
        where = ""
        offset = places[line_number] if line_number < len(places) else line_number
        for function in FUNCTIONS:
            if function_matches(original, function, offset):
                where = function
                break
        # The port's names are the C's without the two underscores.
        where = where.lstrip("_")
        if isinstance(value, tuple) and value[0] == "table":
            STATIC_NAMES.append(
                (name, "double", ("table", value[1], value[2]), where))
        elif isinstance(value, tuple) and value[0] == "alias":
            STATIC_NAMES.append(
                (name, "double", ("alias", value[1]), where))
        else:
            STATIC_NAMES.append((name, "double", value, where))
    out = []
    i = 0
    n = len(text)
    while i < n:
        if text.startswith("//", i):
            j = text.find("\n", i)
            if j < 0:
                break
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i)
            if j < 0:
                break
            out.append(" " * (j + 2 - i))
            i = j + 2
        else:
            out.append(text[i])
            i += 1
    return "".join(out), original


def find_body(text, name):
    """The body of a function, from its signature to its closing brace."""
    pattern = re.compile(r"^[A-Za-z_][\w \*]*\b" + re.escape(name) + r"\s*\(", re.M)
    for match in pattern.finditer(text):
        start = match.start()
        i = text.find("(", match.end() - 1)
        depth = 0
        while i < len(text):
            if text[i] == "(":
                depth += 1
            elif text[i] == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        j = text.find("{", i)
        if j < 0:
            continue
        between = text[i:j]
        if "=" in between or ";" in between:
            continue
        depth = 0
        k = j
        while k < len(text):
            if text[k] == "{":
                depth += 1
            elif text[k] == "}":
                depth -= 1
                if depth == 0:
                    return text[start:k + 1]
            k += 1
    return None


def inside_functions(text):
    """A test for whether a place in the text is inside a function.

    The C's functions are the blocks that are not the file's own, and the
    tables that are inside one of them belong to that function.
    """
    # The places that are inside a function: between the function's
    # signature and the end of its body.
    places = []
    for name in FUNCTIONS:
        matches = list(re.finditer(
            r"^[A-Za-z_][\w \*]*\b" + re.escape(name) + r"\s*\(",
            text, re.M))
        if not matches:
            continue
        # The C declares a function before it defines it, and the
        # declaration is not the definition: the last is.
        match = matches[-1]
        # The body, which is in braces.
        index = text.find("{", match.end())
        if index < 0:
            continue
        depth = 0
        while index < len(text):
            if text[index] == "{":
                depth += 1
            elif text[index] == "}":
                depth -= 1
                if depth == 0:
                    index += 1
                    break
            index += 1
        places.append((match.start(), index))

    def test(where):
        return any(start <= where <= end for start, end in places)

    return test


def function_matches(text, name, where):
    """Whether a place in the text is inside a function of that name.

    The C's names carry two underscores where the port's carry none, so
    both are tried.
    """
    matches = list(re.finditer(
        r"^[A-Za-z_][\w \*]*\b__" + re.escape(name) + r"\s*\(",
        text, re.M))
    if not matches:
        matches = list(re.finditer(
            r"^[A-Za-z_][\w \*]*\b" + re.escape(name) + r"\s*\(",
            text, re.M))
    if not matches:
        return False
    match = matches[-1]
    index = text.find("{", match.end())
    if index < 0:
        return False
    depth = 0
    while index < len(text):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                # The place is in the function when it is between the
                # function's signature and the end of its body. The
                # `#define`s are written before the body opens, and they
                # belong to the function.
                return match.start() <= where <= index
        index += 1
    return False


def find_tables(text):
    """The `static const double` tables and constants, with their values.

    The C writes them over several lines, and sometimes several to a
    declaration, so the whole declaration is read first and then split.
    Only the ones that are outside every function are taken, because the
    ones inside a function belong to that function.
    """
    tables = {}
    pattern = re.compile(r"static const double\s+([^;]*);", re.S)
    inside = inside_functions(text)
    for match in pattern.finditer(text):
        if inside(match.start()):
            continue
        declaration = match.group(1)
        # An array.
        array = re.match(r"\s*(\w+)\s*\[\s*\]\s*=\s*\{(.*)\}\s*$", declaration, re.S)
        if array:
            name = array.group(1)
            values = re.findall(
                r"[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?", array.group(2))
            tables[name] = values
            continue
        # One or more named values. The comments are taken out, because
        # a comment may hold a comma.
        declaration = re.sub(r"/\*.*?\*/", " ", declaration, flags=re.S)
        declaration = re.sub(r"//[^\n]*", " ", declaration)
        for part in declaration.split(","):
            single = re.match(
                r"\s*(\w+)\s*=\s*([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)\s*$",
                part)
            if single:
                tables[single.group(1)] = single.group(2)
    return tables


def add_braces_to_bare_ifs(text):
    """Give the C's braces-less `if` a block.

    The C writes `if (test) statement;` and Rust writes `if test {
    statement; }`, so the statement has to be found and wrapped. The
    port's own `if`, which the ternary has already written, is left
    alone: it already has a block.
    """
    """Give the C's braces-less `if` and `else` a block.

    The C writes `if (test) statement;` and Rust writes `if test {
    statement; }`, so the statement has to be found and wrapped.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        # A word inside brackets is not a keyword of the C's: the ternary
        # is written as an `if` in Rust, and it is inside brackets. The
        # next bracket or keyword is found, and the brackets are skipped.
        # A keyword and a bracket are both looked for, and whichever is
        # nearer is taken. A keyword at the scan's own place comes first,
        # so that the C's `if (test)` is not read as a bracket.
        keyword = re.compile(r"\b(if|else|while)\b").search(text, i)
        bracket = re.compile(r"[(\[]").search(text, i)
        if not keyword and not bracket:
            out.append(text[i:])
            break
        if keyword and (not bracket or keyword.start() < bracket.start()):
            match = keyword
        else:
            depth = 0
            j = bracket.start()
            while j < n:
                if text[j] in "([":
                    depth += 1
                elif text[j] in ")]":
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
            out.append(text[i:j])
            i = j
            continue
        # The keyword, and the condition that may follow it. The
        # condition is in brackets, and it may hold a cast, which has
        # brackets of its own, so the brackets are found by their
        # balance.
        match = re.compile(r"\b(if|else|while)\b").search(text, i)
        if not match:
            out.append(text[i:])
            break
        keyword = match.group(1)
        j = match.end()
        while j < n and text[j] in " \t\n":
            j += 1
        condition = ""
        if keyword != "else" and j < n and text[j] == "(":
            depth = 0
            k = j
            while k < n:
                if text[k] == "(":
                    depth += 1
                elif text[k] == ")":
                    depth -= 1
                    if depth == 0:
                        k += 1
                        break
                k += 1
            condition = text[j:k]
            j = k
        # A `for` is copied as it is: the `for` translation does it
        # afterwards, and it needs the C's own header.
        for_match = re.compile(r"\bfor\s*\(").search(text, i)
        if for_match and for_match.start() == i:
            j = i
            depth = 0
            while j < n:
                if text[j] == "(":
                    depth += 1
                elif text[j] == ")":
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
            out.append(text[i:j])
            i = j
            continue
        # The text before the keyword is copied as it is, and the keyword
        # with its own condition.
        out.append(text[i:match.start()])
        out.append(keyword)
        out.append(condition)
        i = j
        if keyword == "else":
            # The C's `else` has no condition of its own, but it may be
            # followed by another `if`, which Rust writes the same way.
            out.append(" ")
            while i < n and text[i] in " \t\n":
                i += 1
            if text[i:i + 2] == "if":
                # The `else if`: the `if` and its condition are taken
                # with the `else`, and the statement is found after.
                j = i + 2
                while j < n and text[j] in " \t\n":
                    j += 1
                if j < n and text[j] == "(":
                    depth = 0
                    while j < n:
                        if text[j] == "(":
                            depth += 1
                        elif text[j] == ")":
                            depth -= 1
                            if depth == 0:
                                j += 1
                                break
                        j += 1
                    out.append(text[i:j])
                    i = j
                    continue
        # The statement, which is one thing or a block.
        while i < n and text[i] in " \t\n":
            i += 1
        if i < n and text[i] == "{":
            # A block already: its own contents may hold a brace-less
            # `if`, so the pass is run over them again.
            depth = 0
            j = i
            while j < n:
                if text[j] == "{":
                    depth += 1
                elif text[j] == "}":
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
            block = text[i:j]
            out.append(add_braces_to_bare_ifs(block))
            i = j
            continue
        # A single statement.
        j = i
        depth = 0
        while j < n:
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
            elif text[j] == ";" and depth == 0:
                j += 1
                break
            j += 1
        out.append("{ " + text[i:j] + " }")
        i = j
    return "".join(out)


def translate_for_header(header, body):
    """One C's `for (init; test; step)` as Rust's `loop`.

    The C's loop is

        init
        while test {
            body
            step
        }

    and the port's is the same, for the C's arithmetic on the loop's
    own variables is the arithmetic the port does.
    """
    match = re.match(r"\s*for\s*\((.*)\)", header, re.S)
    if not match:
        return header
    parts = match.group(1).split(";")
    if len(parts) != 3:
        return header
    initial, test, step = (part.strip() for part in parts)
    # The initialisation and the step may be more than one statement,
    # which the C separates with a comma.
    initial = initial.replace(", ", "; ")
    step = step.replace(", ", "; ")
    # The step is a statement, and the C's `i++` is the port's
    # `i += 1`.
    step = re.sub(r"\+\+", " += 1", step)
    step = re.sub(r"--", " -= 1", step)
    # The condition is in brackets, for the rule that gives the C's
    # braces-less `if` and `while` a block reads it there. The whole is
    # in a block of its own, for the C's `for` is a statement and the
    # port's is too.
    return f"{{ {initial}; while ({test}) {{ {body} {step} }} }}"


def translate_for_loops(text):
    """Every C's `for` in the text, with its body, as Rust's `loop`.

    The C's loop is

        for (init; test; step) body

    and the port's is

        loop { init; while (test) { body step } }

    for the C's arithmetic on the loop's own variables is the arithmetic
    the port does, and the step comes after the body as it does in the C.

    The loops are taken one at a time, from the last to the first, for
    the loop that is inside another comes after it, and the body of a
    loop is the text that is inside it.
    """
    while True:
        founds = list(re.finditer(r"\bfor\s*\(", text))
        if not founds:
            return text
        found = founds[-1]
        # The header, which is found by its brackets.
        j = found.end()
        depth = 1
        while j < len(text) and depth:
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
            j += 1
        header = text[found.start():j]
        # The body, which is in braces or is one statement.
        k = j
        while k < len(text) and text[k] in " \t\n":
            k += 1
        if k < len(text) and text[k] == "{":
            depth = 0
            end = k
            while end < len(text):
                if text[end] == "{":
                    depth += 1
                elif text[end] == "}":
                    depth -= 1
                    if depth == 0:
                        end += 1
                        break
                end += 1
            body = text[k + 1:end - 1]
        else:
            end = text.find(";", k)
            if end < 0:
                end = len(text)
            else:
                end += 1
            body = text[k:end]
        text = (
            text[:found.start()]
            + translate_for_header(header, body)
            + text[end:]
        )


def translate_gotos(text):
    """The C's `goto` and its label.

    The C jumps back to a label, and the port writes the labelled part
    as a loop with a label on it, so that the jump becomes a `continue`
    and the loop's own condition ends it.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        # A label of the C's own, which is a name on a line of its own.
        # The `default:` of a `switch` is not one of these, for the
        # `switch` is not a jump but a choice, and the port reads it as
        # a choice.
        match = re.compile(
            r"^([ \t]*)(?!default\b)(\w+):\s*$", re.M).search(text, i)
        if not match:
            out.append(text[i:])
            break
        out.append(text[i:match.start()])
        label = match.group(2)
        # The label becomes a Rust loop with a label on it, and the jump
        # back to it becomes a `continue`. The loop's own end is the end
        # of the block the label is in, so the label's block is closed
        # and the loop is given the rest of it.
        out.append(f"{match.group(1)}'{label}: loop {{")
        i = match.end()
        # The body, which runs to the `goto` that jumps back to it.
        goto = re.compile(r"\bgoto\s+" + label + r"\s*;").search(text, i)
        if not goto:
            out.append(text[i:])
            out.append("}")
            break
        out.append(text[i:goto.start()])
        out.append("continue;")
        i = goto.end()
        # The blocks that are open at the `goto`, for the port's loop
        # holds them and they are closed within it.
        depth = 0
        for character in text[match.end():goto.start()]:
            if character == "{":
                depth += 1
            elif character == "}":
                depth -= 1
        # Their closes, which the C writes after the `goto`, and which
        # the port writes within the loop.
        while depth > 0 and i < n:
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                out.append("}")
            i += 1
        # And the rest of the block the label is in, which is the loop's
        # body, for the C's own end of it is a return and the port's
        # loop must hold it.
        rest = text[i:]
        cut = rest.rfind("}")
        if cut < 0:
            cut = len(rest)
        out.append(rest[:cut])
        # And the loop's own end, and the block's own end.
        out.append("}")
        out.append(rest[cut:])
        break
    return "".join(out)


def translate_switches(text):
    """The C's `switch` as Rust's `match`.

    The C's cases fall through, and Rust's do not, so each case is
    written as its own arm with the bodies in the order the C has them.
    The C's `break` is Rust's `break` in a loop, so it is dropped, and
    the fall-through is kept by the arms.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        match = re.compile(r"\bswitch\s*\(").search(text, i)
        if not match:
            out.append(text[i:])
            break
        out.append(text[i:match.start()])
        j = match.end()
        depth = 1
        while j < n and depth:
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
            j += 1
        subject = text[match.end():j - 1].strip()
        # The body, which is in braces.
        k = j
        while k < n and text[k] in " \t\n":
            k += 1
        if k >= n or text[k] != "{":
            out.append(text[match.start():j])
            i = j
            continue
        depth = 0
        end = k
        while end < n:
            if text[end] == "{":
                depth += 1
            elif text[end] == "}":
                depth -= 1
                if depth == 0:
                    end += 1
                    break
            end += 1
        body = text[k + 1:end - 1]
        # The cases, which are `case X:` or `default:` and a body.
        arms = []
        rest = body
        while rest:
            case = re.match(
                r"\s*(?:case[ \t]+([^:\n]+):|default:)", rest)
            if not case:
                arms.append(("__rest__", rest))
                break
            value = case.group(1)
            rest = rest[case.end():]
            # The body, which runs to the next case or to the end. The
            # C writes a case on its own line, and the port finds it
            # there, so that a `:` inside the body is not mistaken for
            # the end of a case.
            # The next case, which the C writes on its own line.
            nxt = re.search(
                r"\n[ \t]*(case[ \t]+|default[ \t]*:)", rest)
            if nxt:
                start = nxt.start()
            else:
                start = len(rest)
            arm = rest[:start]
            # The C's `break` leaves the switch, and Rust's match arm
            # ends by itself, so the `break` is dropped.
            arm = re.sub(r"\bbreak\s*;", "", arm)
            arms.append((value, arm))
            rest = rest[start:]
        out.append(f"match {subject} {{")
        # The C's `default:` is the arm that catches everything, and
        # Rust's catch-all must come last, so it is held back.
        catch_all = None
        # A case with no body of its own falls through to the case that
        # follows it, and the port's arm holds the two of them.
        pending = []
        for value, arm in arms:
            if value == "__rest__" or value is None:
                catch_all = arm
                continue
            if arm.strip():
                out.append(
                    f"    {' | '.join(pending + [value])} => {{ {arm} }}")
                pending = []
            else:
                pending.append(value)
        if pending:
            # The cases that fall through to the end, which is the C's
            # `default:` when there is one.
            if catch_all is None:
                out.append(f"    {' | '.join(pending)} => {{}}")
            else:
                out.append(
                    f"    {' | '.join(pending)} => {{ {catch_all} }}")
                catch_all = None
        if catch_all is None:
            # The C's `switch` with no `default:` does nothing when no
            # case matches, and the port's `match` must be told that.
            out.append("    _ => {}")
        else:
            out.append(f"    _ => {{ {catch_all} }}")
        out.append("}")
        # The C's `switch` is a statement, and the port's `match` is one
        # too, for it is a choice and not a value.
        out.append(";")
        i = end
    return "".join(out)


def condition_start(text, at):
    """Where the condition of a ternary begins.

    The condition is the expression that ends at the `?`, and the scan
    walks back over it: a name, a number, a call, or something in
    brackets.
    """
    i = at
    while i > 0:
        while i > 0 and text[i - 1] in " \t\n":
            i -= 1
        if i == 0:
            return 0
        character = text[i - 1]
        if character == ")":
            # The bracket must match one that is open, or the scan has
            # come past the end of the condition.
            depth = 0
            j = i - 1
            matched = False
            while j >= 0:
                if text[j] == ")":
                    depth += 1
                elif text[j] == "(":
                    depth -= 1
                    if depth == 0:
                        matched = True
                        break
                j -= 1
            if not matched:
                return i
            i = j
            while i > 0 and (text[i - 1].isalnum() or text[i - 1] in "_:."):
                i -= 1
            continue
        if character.isalnum() or character in "_.":
            # The word may be `return`, which is not part of the
            # condition.
            j = i
            while j > 0 and (text[j - 1].isalnum() or text[j - 1] in "_."):
                j -= 1
            if text[j:i] == "return":
                return j
            i = j
            continue
        if character == "=":
            # The condition is the right-hand side of an assignment, and
            # the assignment is not part of it.
            return i
        if character in "<>=!&|+-*/%":
            # The condition ends in an operation, and the scan walks back
            # over it, and over the name it works on.
            i -= 1
            while i > 0 and text[i - 1] in " \t\n":
                i -= 1
            if i > 0 and text[i - 1] == ")":
                continue
            if i > 0 and (text[i - 1].isalnum() or text[i - 1] in "_."):
                i -= 1
                while i > 0 and (text[i - 1].isalnum() or text[i - 1] in "_."):
                    i -= 1
            continue
        return i
    return 0


def translate_ternary(text):
    """The C's `condition ? first : second` as Rust's `if`.

    The condition and the two branches are found by their brackets, and
    the whole expression is written out as Rust's `if`, which is what the
    C means by it.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        question = text.find("?", i)
        if question < 0:
            out.append(text[i:])
            break
        # The `?` may be the C's own, or part of a name.
        if question > 0 and (text[question - 1].isalnum() or
                             text[question - 1] in "_."):
            out.append(text[i:question + 1])
            i = question + 1
            continue
        # The whole expression may be in brackets, and then the `?` is
        # inside them, and the condition is what is inside them.
        enclosing_open = -1
        enclosing_close = -1
        depth = 0
        index = question
        while index > 0:
            index -= 1
            if text[index] == ")":
                depth += 1
            elif text[index] == "(":
                if depth == 0:
                    enclosing_open = index
                    break
                depth -= 1
        if enclosing_open >= 0:
            # The end of the brackets.
            depth = 0
            enclosing_close = question
            while enclosing_close < n:
                if text[enclosing_close] == "(":
                    depth += 1
                elif text[enclosing_close] == ")":
                    if depth == 0:
                        break
                    depth -= 1
                enclosing_close += 1
            # The brackets are the whole expression only when the `:` is
            # inside them as well, which is what tells the two apart.
            between = text[question:enclosing_close]
            if ":" not in between:
                enclosing_open = -1
                enclosing_close = -1
        # The condition, which ends at the `?`. The brackets the
        # condition is in are the C's own, and they are taken away.
        start = condition_start(text, question)
        condition = text[start:question].strip()
        if (condition.startswith("(") and condition.endswith(")")
                and condition.count("(") == condition.count(")")):
            condition = condition[1:-1].strip()
            # The brackets are at the front of the condition, and the
            # text that is kept is what comes before them.
            while start > 0 and text[start - 1] in " \t\n":
                start -= 1
            if start > 0 and text[start - 1] == "(":
                start -= 1
        # The C's `return` is before the condition and not part of it.
        prefix = ""
        while condition.startswith("return"):
            prefix += "return "
            condition = condition[len("return"):].strip()
        # The first branch, which runs to the `:`.
        k = question + 1
        while k < n and text[k] in " \t\n":
            k += 1
        begin = k
        depth = 0
        while k < n:
            if text[k] in "([{":
                depth += 1
            elif text[k] in ")]}":
                depth -= 1
            elif text[k] == ":" and depth == 0:
                if text[k:k + 2] == "::":
                    k += 2
                    continue
                break
            k += 1
        first = text[begin:k].strip()
        k += 1
        while k < n and text[k] in " \t\n":
            k += 1
        # The second branch, which runs to the end of the expression.
        begin = k
        depth = 0
        while k < n:
            if text[k] in "([{":
                depth += 1
            elif text[k] in ")]}":
                if depth == 0:
                    break
                depth -= 1
            elif text[k] == ";" and depth == 0:
                break
            k += 1
        second = text[begin:k].strip()
        if enclosing_open >= 0:
            # The brackets the whole expression is in are the C's own,
            # and the port's `if` has brackets of its own.
            inner = text[enclosing_open + 1:question]
            rest = text[question + 1:enclosing_close]
            colon = rest.index(":")
            first = rest[:colon].strip()
            second = rest[colon + 1:].strip()
            condition = inner.strip()
            prefix = ""
            while condition.startswith("return"):
                prefix += "return "
                condition = condition[len("return"):].strip()
            out.append(text[i:enclosing_open])
            out.append(
                f"{prefix}if ({condition}) {{ {first} }} else {{ {second} }}")
            i = enclosing_close + 1
            continue
        out.append(text[i:start])
        out.append(
            f"{prefix}if ({condition}) {{ {first} }} else {{ {second} }}")
        i = k
    return "".join(out)


def translate_bare_ternaries(text):
    """The C's ternary whose condition is not in brackets.

    The C writes `x > 0 ? a : b`, and the port writes it as Rust's `if`.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        found = re.compile(r"\?").search(text, i)
        if not found:
            out.append(text[i:])
            break
        # The `?` may be inside brackets, and then the whole expression
        # is those brackets: the condition and the branches are inside
        # them, and the brackets go with the `if`.
        opening = found.start()
        depth = 0
        index = found.start()
        while index > 0:
            index -= 1
            if text[index] == ")":
                depth += 1
            elif text[index] == "(":
                if depth == 0:
                    opening = index
                    break
                depth -= 1
        inside = opening < found.start()
        if not inside:
            # The brackets before the `?` may hold the condition, and
            # then the condition is what is inside them.
            if opening < found.start():
                inner = text[opening + 1:found.start()]
                if inner.strip() and "?" not in inner:
                    condition = inner.strip()
                    start = opening
                else:
                    condition = ""
            else:
                condition = ""
        if inside:
            # The end of the brackets.
            closing = found.end()
            depth = 0
            while closing < n:
                if text[closing] == "(":
                    depth += 1
                elif text[closing] == ")":
                    if depth == 0:
                        break
                    depth -= 1
                closing += 1
            inner = text[opening + 1:closing]
            question = inner.index("?")
            condition = inner[:question].strip()
            rest = inner[question + 1:]
            colon = rest.index(":")
            first = rest[:colon].strip()
            second = rest[colon + 1:].strip()
            out.append(text[i:opening])
            out.append(
                f"if ({condition}) {{ {first} }} else {{ {second} }}")
            i = closing + 1
            continue
        start = condition_start(text, found.start())
        condition = text[start:found.start()].strip()
        # The condition may be in brackets, which are the C's own, and
        # the port's brackets hold the same.
        if (condition.startswith("(") and condition.endswith(")")
                and condition.count("(") == condition.count(")")):
            condition = condition[1:-1].strip()
        # The C's `return` is before the condition and not part of it,
        # and the port's `return` takes the whole of the expression, so
        # it is put back in front of the `if`.
        prefix = ""
        while condition.startswith("return"):
            prefix += "return "
            condition = condition[len("return"):].strip()
        k = found.end()
        # The first branch, which runs to the `:`. A `::` is not the
        # end of a branch: it is the scope operator of a name.
        start = k
        depth = 0
        while k < n:
            if text[k] in "([{":
                depth += 1
            elif text[k] in ")]}":
                depth -= 1
            elif text[k] == ":" and depth == 0:
                if text[k:k + 2] == "::":
                    k += 2
                    continue
                break
            k += 1
        first = text[start:k].strip()
        k += 1
        while k < n and text[k] in " \t\n":
            k += 1
        start = k
        depth = 0
        while k < n:
            if text[k] in "([{":
                depth += 1
            elif text[k] in ")]}":
                depth -= 1
            elif text[k] == ";" and depth == 0:
                break
            k += 1
        second = text[start:k].strip()
        out.append(text[i:start])
        out.append(
            f"{prefix}if ({condition}) {{ {first} }} else {{ {second} }}")
        i = k
    return "".join(out)


def translate_ternaries(text):
    """The C's `cond ? a : b` as Rust's `if cond { a } else { b }`.

    The branches are found by their brackets, because a branch may hold
    a cast of its own, which has brackets too.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        # The condition, in its own brackets. The brackets are found by
        # their balance, so that a ternary inside them is left for the
        # scan to find.
        if text[i] != "(":
            out.append(text[i])
            i += 1
            continue
        depth = 0
        j = i
        while j < n:
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    j += 1
                    break
            j += 1
        condition = text[i + 1:j - 1]
        k = j
        while k < n and text[k] in " \t\n":
            k += 1
        if k >= n or text[k] != "?":
            # The brackets hold no ternary of their own, so they are
            # copied as they are, and the search carries on inside them.
            out.append(text[i:i + 1])
            out.append(translate_ternaries(text[i + 1:j - 1]))
            out.append(text[j - 1:j])
            i = j
            continue
        k += 1
        while k < n and text[k] in " \t\n":
            k += 1
        # The first branch, which runs to the `:`. A `::` is not the
        # end of a branch: it is the scope operator of a name.
        start = k
        depth = 0
        while k < n:
            if text[k] in "([{":
                depth += 1
            elif text[k] in ")]}":
                depth -= 1
            elif text[k] == ":" and depth == 0:
                if text[k:k + 2] == "::":
                    k += 2
                    continue
                break
            k += 1
        first = text[start:k].strip()
        k += 1
        while k < n and text[k] in " \t\n":
            k += 1
        # The second branch, which runs to the end of the statement.
        start = k
        depth = 0
        while k < n:
            if text[k] in "([{":
                depth += 1
            elif text[k] in ")]}":
                depth -= 1
            elif text[k] == ";" and depth == 0:
                break
            k += 1
        second = text[start:k].strip()
        out.append(
            f"(if {condition} {{ {first} }} else {{ {second} }})")
        i = k
    return "".join(out)


def declared_types(text):
    """The type of each variable the C declares, by its name.

    The C's words are unsigned, and the C's variables are often signed,
    and the port's macros give a word, so the macro's result is cast to
    the type the variable was declared with.
    """
    types = {}
    pattern = re.compile(
        r"(?m)^\s*(double|int32_t|uint32_t|uint64_t|int|unsigned)\s+"
        r"([^;{}]+);")
    for match in pattern.finditer(text):
        rust_type = rust_type_of(match.group(1))
        # The names, which are before the value of their own, for the
        # value may hold commas of its own.
        head = match.group(2).split("=")[0]
        for part in head.split(","):
            name = part.strip().split("[")[0].strip()
            if re.match(r"^\w+$", name):
                types[name] = rust_type
    return types


def substitute_macros(text, types=None):
    """The C's macros that take a double apart and put it together.

    An argument may hold a cast, which has brackets of its own, so the
    arguments are found by their brackets.
    """
    macros = {
        # The C's macros are statements, and they write into the variable
        # they are given. The port does the same, so that a name means
        # the same thing on both sides of the line.
        "EXTRACT_WORDS": (
            "({0}, {1}) = (extract_words({2}).0 as {3}, "
            "extract_words({2}).1 as {4});", 3),
        "EXTRACT_WORDS_U32": (
            "({0}, {1}) = (extract_words({2}).0 as {3}, "
            "extract_words({2}).1 as {4});", 3),
        "GET_HIGH_WORD": ("{0} = get_high_word({1}) as {2};", 2),
        "GET_LOW_WORD": ("{0} = get_low_word({1}) as {2};", 2),
        "LOW_WORD": ("{0} = get_low_word({1}) as {2};", 2),
        "INSERT_WORDS": ("{0} = insert_words(({1}) as u32, ({2}) as u32);", 3),
        "SET_HIGH_WORD": ("set_high_word(&mut {0}, ({1}) as u32);", 2),
        "SET_LOW_WORD": ("set_low_word(&mut {0}, ({1}) as u32);", 2),
    }
    out = []
    i = 0
    n = len(text)
    # The macros are looked for together, and the nearest one is taken, so
    # that the text is read from left to right.
    pattern = re.compile(
        r"\b(" + "|".join(macros) + r")\s*\(")
    while i < n:
        found = pattern.search(text, i)
        if not found:
            out.append(text[i:])
            break
        at = found.start()
        name = found.group(1)
        out.append(text[i:at])
        # The arguments, which are found by their brackets.
        j = found.end()
        depth = 1
        start = j
        while j < n:
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        arguments = text[start:j]
        j += 1
        # The arguments are split on the commas that are not in
        # brackets.
        parts = []
        depth = 0
        last = 0
        for k, character in enumerate(arguments):
            if character in "([":
                depth += 1
            elif character in ")]":
                depth -= 1
            elif character == "," and depth == 0:
                parts.append(arguments[last:k].strip())
                last = k + 1
        parts.append(arguments[last:].strip())
        if len(parts) != macros[name][1]:
            # Not a call of this macro: it is the C's own text, and it is
            # copied as it is.
            out.append(text[at:j])
            i = j
            continue
        # The argument may hold braces of its own, which the format
        # would take for its own.
        if types:
            # The word the macro gives is cast to the type the variable
            # was declared with, as the C's own assignment does.
            # The name the macro is given is its first argument, and
            # the value it works on is its second.
            target = parts[0]
            rust_type = types.get(target) or "Int32"
            if name in ("GET_HIGH_WORD", "GET_LOW_WORD"):
                # The word the macro gives is cast to the type the
                # variable was declared with, as the C's own assignment
                # does. The cast is on the result, which is the whole
                # of the macro's right-hand side.
                out.append(macros[name][0].format(
                    target, parts[1], rust_type))
                i = j
                continue
            if name in ("EXTRACT_WORDS", "EXTRACT_WORDS_U32"):
                # The two words are given to the two variables, each
                # cast to the type it was declared with.
                first = types.get(parts[0]) or "Int32"
                second = types.get(parts[1]) or "Int32"
                out.append(macros[name][0].format(
                    parts[0], parts[1], parts[2], first, second))
                i = j
                continue
        out.append(macros[name][0].format(
            *[part.replace("{", "{{").replace("}", "}}")
              for part in parts]))
        i = j
        # The macro's own text ends with a semicolon, and the C's
        # statement has one too, so the C's is taken away.
        if text[i:i + 1] == ";":
            i += 1
    return "".join(out)


def take_group(match, where=""):
    """The C's `static` group, taken out and written out as constants.

    A group is one type and several names, and the names may be tables
    with values of their own, so the parts are split on the commas that
    are not in brackets.
    """
    kind = match.group(1)
    body_text = match.group(2)
    # The comments are taken out, because a comment may hold a comma.
    body_text = re.sub(r"/\*.*?\*/", " ", body_text, flags=re.S)
    body_text = re.sub(r"//[^\n]*", " ", body_text)
    parts = []
    depth = 0
    last = 0
    for index, character in enumerate(body_text):
        if character in "([{":
            depth += 1
        elif character in ")]}":
            depth -= 1
        elif character == "," and depth == 0:
            parts.append(body_text[last:index])
            last = index + 1
    parts.append(body_text[last:])
    for part in parts:
        part = part.strip()
        table = re.match(
            r"^(\w+)\s*\[\s*(\d*)\s*\]\s*=\s*\{([^}]*)\}$", part, re.S)
        if table:
            values = re.findall(
                r"[-+]?(?:0[xX][0-9A-Fa-f]+|\d+(?:\.\d*)?(?:[eE][-+]?\d+)?)",
                table.group(3))
            STATIC_NAMES.append((table.group(1), kind, ("array", values), where))
            continue
        single = re.match(
            r"^(\w+)\s*=\s*([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)$",
            part)
        if single:
            STATIC_NAMES.append((single.group(1), kind, single.group(2), where))
    return ""


def rust_type_of(c_type):
    """The C's type as Rust's.

    The C's 32-bit words are unsigned, and the port keeps them signed,
    for the C's own two's complement holds the same bits either way.
    Where the C's arithmetic on them is on the unsigned word, the port
    writes the conversion out.
    """
    return {"double": "f64", "int32_t": "Int32", "uint32_t": "Int32",
            "uint64_t": "Uint64", "int": "Int32", "unsigned": "Int32"}.get(
                c_type, c_type)


def rust_number(text, kind):
    """A C literal as a Rust one, of the kind the table holds."""
    text = text.strip()
    if text.lower().startswith(("0x", "-0x")):
        # A hex word, which is kept as it is: it is a bit pattern, not a
        # number.
        return text
    if kind in ("int32_t", "uint32_t", "int"):
        return str(int(float(text)))
    return rust_double(text)


def rust_double(text):
    """A C double literal as a Rust one."""
    text = text.strip()
    if text.startswith("0x") or text.startswith("-0x"):
        # A C hexadecimal float, such as 0x1.0p-1.
        sign = ""
        if text.startswith("-"):
            sign = "-"
            text = text[1:]
        mantissa, _, exponent = text.partition("p")
        value = float.fromhex(mantissa)
        if exponent:
            return f"({sign}{value!r} * 2f64.powi({int(exponent)}))"
        return f"({sign}(0x{mantissa[2:]}) as f64)"
    return repr(float(text))


# The `static` locals that were taken out of the functions, which are
# written at the top of the module.
STATIC_NAMES = []


def translate_body(text, tables, where=""):
    """The C++ as Rust."""
    out = text

    # The C's `for` headers are taken out of the text, because the
    # ternary below has the same shape and would read them, and because
    # the loop's step is a statement and not an expression.
    headers = []

    # The C's `for` loops, which are Rust's `loop`, and which are taken
    # before the ternary below, for the ternary has the same shape and
    # would read them.
    out = translate_for_loops(out)

    # The C's `static_cast<T>(x)`, whose argument may hold brackets of
    # its own, so the argument is found by counting them.
    def translate_static_casts(text):
        out = []
        i = 0
        n = len(text)
        pattern = re.compile(r"static_cast<\s*(\w+)\s*>\s*\(")
        while i < n:
            found = pattern.search(text, i)
            if not found:
                out.append(text[i:])
                break
            out.append(text[i:found.start()])
            depth = 1
            j = found.end()
            start = j
            while j < n:
                if text[j] == "(":
                    depth += 1
                elif text[j] == ")":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            argument = translate_static_casts(text[start:j])
            out.append(
                f"(({argument}) as {rust_type_of(found.group(1))})")
            i = j + 1
        return "".join(out)

    out = translate_static_casts(out)
    # The casts inside the macros, where the type is written without a
    # `static_cast`.
    out = re.sub(r"\(int32_t\)", "as Int32", out)


    # The C's local declarations, which are one type and a list of names,
    # and which may have an array among them.
    def declaration(match):
        kind = match.group(1)
        names = match.group(2)
        rust_type = rust_type_of(kind)
        out = []
        for part in names.split(","):
            part = part.strip()
            array = re.match(r"^(\w+)\[(\w*)\]$", part)
            if array:
                size = array.group(2) or "0"
                zero = "0.0" if rust_type == "f64" else "0"
                out.append(
                    f"let mut {array.group(1)}: [{rust_type}; {size}] = "
                    f"[{zero}; {size}];")
            else:
                # A name with a value of its own, which the C writes as
                # `name = value`.
                valued = re.match(r"^(\w+)\s*=\s*(.+)$", part)
                if valued:
                    value = valued.group(2)
                    if rust_type == "f64" and re.fullmatch(
                            r"[-+]?\d+", value):
                        value = value + ".0"
                    out.append(
                        f"let mut {valued.group(1)}: {rust_type} = {value};")
                else:
                    # The C's local is written before it is read, and Rust
                    # will not take a variable that has no value yet, so
                    # it is given the C's own zero.
                    if rust_type == "f64":
                        out.append(f"let mut {part}: f64 = 0.0;")
                    elif rust_type == "Uint32":
                        out.append(f"let mut {part}: Uint32 = 0;")
                    elif rust_type == "Uint64":
                        out.append(f"let mut {part}: Uint64 = 0;")
                    else:
                        out.append(f"let mut {part}: {rust_type} = 0;")
        return " ".join(out)

    # The types of the variables are gathered before the declarations are
    # changed, so that the C's own type names are found.
    types = declared_types(out)

    # The parameters, which the C writes as `type name`, and which the
    # port writes as `name: type`.
    signature = re.search(
        r"\b(?:double|int32_t|uint32_t|uint64_t|int|unsigned)\s+\w+\s*"
        r"\(([^)]*)\)",
        out)
    if signature:
        for part in signature.group(1).split(","):
            part = part.strip()
            parameter = re.match(
                r"^(?:const\s+)?(\w+)\s*\*?\s*(\w+)$", part)
            if parameter:
                types[parameter.group(2)] = rust_type_of(
                    parameter.group(1))

    out = re.sub(
        r"(?<!static )(?<!volatile )(?<!const )"
        r"\b(double|int32_t|uint32_t|uint64_t|int|unsigned)\s+"
        r"([^;{}]+);",
        declaration, out)

    # The macros come after the declarations, and a macro's call is
    # turned into a Rust statement.
    out = substitute_macros(out, types)

    # The functions that give a number back say so, which the C's
    # signatures already say.
    # The C's `ix & 0x7FFFFFFF` and the like, where the word is signed
    # and the mask is not, are the C's own arithmetic on unsigned words.
    # The mask takes the word's own kind, which is the C's own
    # conversion of the constant to the word's kind.
    # The words the C declares, which the rules below work on.
    words = "|".join(
        name for name, kind in types.items() if kind == "Int32")

    def mask_of(match):
        word = match.group(1)
        literal = match.group(2)
        if int(literal, 16) >= 0x80000000:
            # The C's literal is an unsigned word, and the C's arithmetic
            # on it is on the unsigned word.
            return f"({word} as u32) & ({literal}u32)"
        return f"{word} & ({literal} as i32)"
    out = re.sub(
        r"\b(" + words + r") & (0x[0-9A-Fa-f]+)\b",
        mask_of,
        out)
    # The C's `low`, `lx` and `ly` hold words, and the C's arithmetic on
    # them is on the word. The port keeps the C's own kinds, and the
    # C's usual arithmetic conversions are written out where the C
    # makes them.
    # The C's integer literal in a double's arithmetic is a double, for
    # the C's own usual arithmetic conversions, and the port writes it
    # as one. A literal that is already a double is left alone.
    for name, kind in types.items():
        if kind != "f64":
            continue
        out = re.sub(
            rf"\b{re.escape(name)}\b ([-+*/]=?|[<>]=?|==|!=) "
            rf"(?<![.\w])(\d+)(?![.\w])",
            lambda m: f"{name} {m.group(1)} {m.group(2)}.0",
            out)
        out = re.sub(
            rf"(?<![.\w])(\d+)(?![.\w])\b ([-+*/]=?|[<>]=?|==|!=) "
            rf"\b{re.escape(name)}\b",
            lambda m: f"{m.group(1)}.0 {m.group(2)} {name}",
            out)
    out = re.sub(
        r"\b(m|xsb) = (\([^;]*?\) \| \([^;]*?\));",
        r"\1 = (\2) as Int32;",
        out)
    out = re.sub(
        r"\b(m|xsb) = (\([^;]*?\));",
        r"\1 = (\2) as Int32;",
        out)
    out = re.sub(
        r"\b(m|xsb) = (\w+ >> 31) & (\d+);",
        r"\1 = ((\2) & (\3)) as Int32;",
        out)
    # The C's word that came from a `GET_HIGH_WORD` is a word, and the
    # C's own `&` on it is on the word.
    out = re.sub(
        r"\b(" + words + r") = (\1 as u32) & (0x[0-9A-Fa-f]+ as i32);",
        r"\1 = ((((\2) as u32) & ((\3) as u32)) as i32);",
        out)
    # The C's comparison of a word with a constant that does not fit in
    # a signed word is on unsigned words, for the C's constant is an
    # unsigned one, and the port does the same. A constant that does fit
    # is a signed one, and the comparison is a signed one.
    def compare(match):
        word, operator, literal = (
            match.group(1), match.group(2), match.group(3))
        if int(literal, 16) >= 0x80000000:
            return f"({word} as u32) {operator} ({literal}u32)"
        return match.group(0)

    out = re.sub(
        r"\b(" + words + r") (<=|>=|<|>|=) (0x[0-9A-Fa-f]{8,})\b",
        compare,
        out)
    out = re.sub(
        r"\b(" + words + r") \| (0x[0-9A-Fa-f]+)\b",
        lambda m: (
            f"({m.group(1)} as u32) | ({m.group(2)}u32)"
            if int(m.group(2), 16) >= 0x80000000
            else f"{m.group(1)} | ({m.group(2)} as i32)"),
        out)
    out = re.sub(
        r"\b(" + words + r") - (0x[0-9A-Fa-f]+)\b",
        lambda m: (
            f"({m.group(1)} as u32).wrapping_sub({m.group(2)}u32)"
            if int(m.group(2), 16) >= 0x80000000
            else f"{m.group(1)}.wrapping_sub({m.group(2)} as i32)"),
        out)


    # The C's words are of two kinds, and the C's own arithmetic
    # converts the signed one to the unsigned one when they are mixed.
    # The port does the same, and the rules are the C's usual arithmetic
    # conversions written out.
    # The C's `ix - 0x3ff00000` is a word, and the C's variable is
    # signed, so the whole of it is unsigned.
    out = re.sub(
        r"\((" + words + ")\.wrapping_sub\((0x[0-9A-Fa-f]+) as i32\)\)",
        r"((\1 as u32).wrapping_sub(\2 as u32))",
        out)
    out = out.replace("i = j - ((high >> 20) & 0x7FF);",
                      "i = j - (((high >> 20) & 0x7FF) as i32);")
    out = out.replace("k += (hx >> 20) - 1023;",
                      "k = k.wrapping_add((hx >> 20) - 1023);")
    out = out.replace(
        "hx = (hx & 0x000FFFFF) | ((0x3FF - i) << 20);",
        "hx = ((hx & 0x000FFFFF) as u32 | ((0x3FF - i as u32) << 20)) as i32;")

    # The C++'s `base::Divide`, which is a division that does not raise
    # the floating point's flags.
    out = re.sub(
        r"base::Divide\s*\(\s*([^,]+),\s*([^)]+)\s*\)",
        r"(\1) / (\2)",
        out)

    # The C's arrays are indexed with a whole number, and Rust wants a
    # `usize`.
    out = re.sub(r"\[(\w+)\s*-\s*1\]", r"[(\1 - 1) as usize]", out)
    # The C's `const_cast`, which is the C's own way of saying that the
    # thing may not be changed, and the port's tables are constants.
    out = re.sub(
        r"\*const_cast<\s*(?:volatile\s+)?\w+\s*\*>\s*\(\s*&?([^()]*?)\s*\)",
        r"\1", out)
    out = re.sub(
        r"const_cast<\s*(?:volatile\s+)?\w+\s*\*>\s*\(\s*&?([^()]*?)\s*\)",
        r"\1", out)

    # The C's `&table[i]` is the address of the entry, and the port has
    # the entry itself.
    out = re.sub(r"&(\w+)\[", r"\1[", out)

    # The C's chained assignment, `a = b = c`, which Rust does not have
    # in the same form.
    def chained(match):
        first, second, value = (
            match.group(1), match.group(2), match.group(3))
        return f"{first} = {value}; {second} = {value};"

    out = re.sub(
        r"\b(\w+(?:\[(?:[^\]\)]+)\])?(?: as usize)?)\s*=\s*"
        r"(\w+(?:\[(?:[^\]\)]+)\])?(?: as usize)?)\s*=\s*([^;]+);",
        chained, out)

    # The C's functions, which Rust has as its own.

    # the C's own words are clear.
    out = out.replace(
        "return (1 - ((hx >> 30) & 2)) * (v - 2.0 * (x - (w * w / (w + v) - r)));",
        "return ((1 - ((hx >> 30) & 2)) as f64) * (v - 2.0 * (x - (w * w / (w + v) - r)));")
    out = out.replace(
        "ix = hx & (0x7FFFFFFF as i32);",
        "ix = ((hx as u32) & (0x7FFFFFFF as u32)) as i32;")
    out = out.replace(
        "iy = hy & (0x7FFFFFFF as i32);",
        "iy = ((hy as u32) & (0x7FFFFFFF as u32)) as i32;")

    # The C's `k_log1p` uses the file's own constants, which the port
    # has named after the file and not after the function.
    for index in range(1, 8):
        out = out.replace(f"Lg{index}_K_LOG1P_1", f"Lg{index}_LOG_1")

    # The C's assignment of a double to an integer, which the C converts
    # by truncating towards zero, and the port writes the conversion out.
    doubles = [
        name for name, kind in types.items() if kind == "f64"
    ]

    def holds_a_double(part):
        if re.search(r"(?<![.\w])\d+\.\d", part):
            return True
        if "as u32" in part:
            # The C's unsigned word, which the C converts when it puts it
            # in a signed one.
            return True
        return any(
            re.search(r"\b" + re.escape(name) + r"\b", part)
            for name in doubles)

    for name, kind in types.items():
        if kind != "Int32":
            continue
        out = re.sub(
            rf"(?m)^(\s*){re.escape(name)} = ([^;]+);",
            lambda m: (
                f"{m.group(1)}{name} = ({m.group(2)}) as Int32;"
                if holds_a_double(m.group(2)) else m.group(0)),
            out)

    # The C's own words are clear.
    out = out.replace(
        "SubWithWraparound(hx, 0x3FF00000)",
        "sub_with_wraparound(hx, 0x3FF00000)")
    out = out.replace(
        "t = k;",
        "t = k as f64;")

    # The C's `k_log1p` uses the file's own constants, which the port
    # has named after the file and not after the function.
    for index in range(1, 8):
        out = out.replace(f"Lg{index}_K_LOG1P_1", f"Lg{index}_LOG_1")


    # the C's own words are clear.
    out = out.replace(
        "i = (hx + 0x95F64) & 0x100000;",
        "i = (((hx as u32) + 0x95F64u32) & 0x100000u32) as Int32;")
    out = out.replace(
        "set_high_word(x, (hx | (i ^ 0x3FF00000)) as u32)",
        "set_high_word(&mut x, ((hx as u32) | ((i as u32) ^ 0x3FF00000u32)))")
    out = out.replace(
        "i = hx.wrapping_sub(0x6147A as i32);",
        "i = ((hx as u32).wrapping_sub(0x6147Au32)) as Int32;")
    # The C's `ix - 0x3ff00000`, which is a signed word, and the C's
    # arithmetic on it is on the signed value.
    out = out.replace(
        "((ix as u32).wrapping_sub(0x3FF00000 as u32))",
        "(ix.wrapping_sub(0x3FF00000))")
    out = out.replace(
        "((ix as u32).wrapping_sub(0x3ff00000 as u32))",
        "(ix.wrapping_sub(0x3ff00000))")
    out = out.replace(
        "((iy as u32).wrapping_sub(0x3FF00000 as u32))",
        "(iy.wrapping_sub(0x3FF00000))")

    # The C's `k += (hx >> 20) - 1023`, which is a signed word.
    out = out.replace(
        "k = ((k as u32).wrapping_add((hx >> 20) - 1023)) as i32;",
        "k = k.wrapping_add((hx >> 20) - 1023);")

    # The C's `hx | low` and `high & 0x7FFFFFFF`, which are on the word.
    out = out.replace(
        "(high & (0x7FFFFFFF as u32))",
        "((high as u32) & (0x7FFFFFFF as u32))")
    # The C's `sign | (hx / 3 + B1)`, which is on the word, for the C
    # converts the signed `hx` to the unsigned one.
    # The C's `sign | (((high & 0x7FFFFFFF) / 3 + B2))`, which is on the
    # word, for the C converts the signed `high` to the unsigned one.
    out = out.replace(
        "((sign | (((high as u32) & (0x7FFFFFFF as u32)) / 3 + B2)) as u32",
        "((sign | ((high & (0x7FFFFFFF as u32)) / 3 + B2)) as u32")
    # The C's arithmetic between a word and a double, which is on the
    # double.
    out = re.sub(
        r"\b(k|i|j|m|n) \* (ln2_lo|ln2_hi|ln2|ln2LO|Ln2)",
        r"(\1 as f64) * (\2)",
        out)
    out = out.replace(
        "y = k + i;", "y = (k + i) as f64;")

    # The C++'s `base::Divide`, which is a division that does not raise
    # the floating point's flags.
    out = re.sub(
        r"base::Divide\s*\(\s*([^,]+),\s*([^)]+)\s*\)",
        r"(\1) / (\2)",
        out)

    # The C's `sign`, which is a word.
    out = out.replace("let mut sign: Uint32 = 0;", "let mut sign: Uint32 = 0;")
    out = re.sub(
        r"\((sign) \| \(\((high) & (0x[0-9A-Fa-f]+)\) / 3 \+ (\w+)\)\)",
        r"((\1 | ((\2 & \3) / 3 + \4)) as u32)",
        out)

    out = re.sub(r"\bfabs\s*\(", "f64::abs(", out)
    out = re.sub(r"\bfloor\s*\(", "f64::floor(", out)
    out = re.sub(r"\bceil\s*\(", "f64::ceil(", out)
    out = re.sub(r"\btrunc\s*\(", "f64::trunc(", out)
    out = re.sub(r"\bsqrt\s*\(", "f64::sqrt(", out)
    out = re.sub(r"(?<![\w:])(?<!double )atan2\s*\(", "f64::atan2(", out)
    out = re.sub(r"\bfmod\s*\(", "f64::rem_euclid(", out)

    # The C's `y` is a double that the port has as an array, and the
    # index is a whole number.


    # An assignment of a whole number to a double is a whole number in
    # the C, and Rust will not take it, so the number is made a float
    # where the variable is a double.
    def whole_number(match):
        name, operator, value = match.group(1), match.group(2), match.group(3)
        if types.get(name) == "f64" and re.fullmatch(r"[-+]?\d+", value):
            value = value + ".0"
        return f"{name} {operator} {value}"

    out = re.sub(
        r"\b(\w+)\s*([-+*]?=)\s*([-+]?\d+)\b(?![.eE])",
        whole_number, out)

    # The same for an element of an array of doubles, which the C's
    # `f[0]`, `f[1]` and the like are.
    def array_element(match):
        name, index, operator, value = (
            match.group(1), match.group(2), match.group(3), match.group(4))
        if re.fullmatch(r"[-+]?\d+", value):
            value = value + ".0"
        return f"{name}[({index}) as usize] {operator} {value}"

    out = re.sub(
        r"\b(\w+)\[\(([^)]+)\)(?: as usize)?\]\s*([-+*]?=)\s*([-+]?\d+)\b(?![.eE])",
        array_element, out)
    out = re.sub(
        r"\b(\w+)\[(\w+)(?: as usize)?\]\s*([-+*]?=)\s*([-+]?\d+)\b(?![.eE])",
        array_element, out)

    # The types.
    out = re.sub(r"\bint32_t\b", "Int32", out)
    out = re.sub(r"\buint32_t\b", "Uint32", out)
    out = re.sub(r"\bint64_t\b", "Int64", out)
    out = re.sub(r"\buint64_t\b", "Uint64", out)

    # The V8 and C++ words.
    out = out.replace("V8_INLINE ", "#[inline] ")
    out = out.replace("V8_WARN_UNUSED_RESULT", "")
    out = out.replace("static inline", "#[inline]")

    # The C's `static volatile` locals, which Rust has as module-level
    # constants. The `volatile` only tells the compiler that the thing
    # may change under the compiler's feet, and the port changes nothing.
    def take_static(match):
        kind = match.group(1)
        name = match.group(2)
        value = match.group(3).strip().rstrip(",").strip()
        STATIC_NAMES.append((name, kind, value, where))
        return ""

    # The C++'s `base::bit_cast<T>(x)`, which is a reinterpretation of
    # the bits. The target says which way round it goes.
    out = re.sub(
        r"base::bit_cast<\s*(\w+)\s*>\s*\(((?:[^()]|\([^()]*\))*)\)",
        lambda m: (
            f"f64::from_bits({m.group(2)})"
            if m.group(1) in ("double", "f64")
            else f"({m.group(2)}).to_bits()"),
        out)

    # The C++'s `NegateWithWraparound`, which is a negation that does not
    # overflow: the smallest number is its own negation.
    out = re.sub(
        r"NegateWithWraparound<\s*\w+\s*>\s*\(\s*([^()]+)\s*\)",
        r"negate_with_wraparound(\1)", out)
    out = re.sub(
        r"SubWithWraparound<\s*\w+\s*>\s*\(\s*([^,()]+),\s*([^()]+)\s*\)",
        r"sub_with_wraparound(\1, \2)", out)

    # The C++'s `const_cast`, which only tells the compiler that the
    # thing may not be changed; the port changes nothing.
    out = re.sub(
        r"\*const_cast<\s*(?:volatile\s+)?\w+\s*\*>\s*\(&([^)]*)\)",
        r"&\1", out)
    out = re.sub(
        r"const_cast<\s*(?:volatile\s+)?\w+\s*\*>\s*\(&([^)]*)\)",
        r"&\1", out)

    # The C++'s quiet NaN, which is what the JavaScript's is.
    out = out.replace(
        "std::numeric_limits<double>::signaling_NaN()", "f64::NAN")
    out = out.replace(
        "std::numeric_limits<double>::quiet_NaN()", "f64::NAN")
    out = out.replace("std::numeric_limits<double>::infinity()", "f64::INFINITY")

    # The C's `scalbn(x, n)`, which is `x * 2^n`.
    out = re.sub(
        r"scalbn\s*\(\s*([^,()]+),\s*([^()]+)\s*\)",
        lambda m: f"({m.group(1)} * 2f64.powi({m.group(2)} as i32))",
        out)

    # The C's own cast, which is a type in brackets and an expression
    # after them.
    out = re.sub(
        r"\(\s*(int32_t|uint32_t|int|double)\s*\)\s*\(([^()]*)\)",
        lambda m: f"(({m.group(2)}) as {rust_type_of(m.group(1))})",
        out)

    # The C's integer literals, which carry their suffixes.
    out = re.sub(r"\b(0[xX][0-9A-Fa-f]+)ULL\b", r"\1u64", out)
    out = re.sub(r"\b(0[xX][0-9A-Fa-f]+)U\b", r"\1u32", out)
    out = re.sub(r"\b(0[xX][0-9A-Fa-f]+)LL\b", r"\1i64", out)
    out = re.sub(r"\b(0[xX][0-9A-Fa-f]+)L\b", r"\1i32", out)

    # The C's bitwise `~`, which Rust writes as `!`.
    out = re.sub(r"~\s*\(", "!(", out)

    # The C's casts.
    for c, r in [
        ("(unsigned)", "as Uint32"),
        ("(int32_t)", "as Int32"),
        ("(uint32_t)", "as Uint32"),
        ("(int64_t)", "as Int64"),
        ("(uint64_t)", "as Uint64"),
        ("(double)", "as f64"),
        ("(int)", "as Int32"),
        ("(unsigned)", "as Uint32"),
    ]:
        out = out.replace(c, r)

    # The function's own signature: the C's types, its pointers, and the
    # order it writes them in.
    out = re.sub(r"^(\s*#\[inline\]\s*)?double (\w+)\s*\(",
                 r"\1fn \2(", out, flags=re.M)
    out = re.sub(r"^Int32 (\w+)\s*\(", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^double (\w+)\s*\(", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^(Int32|int) (\w+)\s*\(", r"fn \2(", out, flags=re.M)
    out = re.sub(r"^Int64 (\w+)\s*\(", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^Uint32 (\w+)\s*\(", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^Uint64 (\w+)\s*\(", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^Int32 (\w+)\s*\{", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^double (\w+)\s*\{", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^void (\w+)\s*\{", r"fn \1(", out, flags=re.M)

    # The parameters, which the C writes as `type name` and Rust writes as
    # `name: type`. A parameter that the C assigns to is a `mut` one.
    assigned = set(re.findall(
        r"\b(\w+)\s*[-+*/%&|^]*=(?!=)", out))
    # A name that the C's macros write into is a `mut` one in the port.
    assigned |= set(re.findall(
        r"set_(?:high|low)_word\(&mut (\w+)", out))
    out = re.sub(r"\bconst int32_t\s*\*\s*(\w+)", r"\1: &[Int32; 66]", out)
    out = re.sub(r"\bconst Int32\s*\*\s*(\w+)", r"\1: &[Int32; 66]", out)
    out = re.sub(r"\bdouble\s*\*\s*(\w+)", r"\1: &mut [f64; 2]", out)
    out = re.sub(r"\bint32_t\s*\*\s*(\w+)", r"\1: &mut [Int32; 1]", out)
    out = re.sub(r"\bint\s*\*\s*(\w+)", r"\1: &mut [Int32; 1]", out)
    out = re.sub(
        r"\bdouble (\w+)(?=[,)])",
        lambda m: (
            f"mut {m.group(1)}: f64" if m.group(1) in assigned
            else f"{m.group(1)}: f64"),
        out)
    out = re.sub(
        r"\bint32_t (\w+)(?=[,)])",
        lambda m: (
            f"mut {m.group(1)}: Int32" if m.group(1) in assigned
            else f"{m.group(1)}: Int32"),
        out)
    out = re.sub(
        r"\bint (\w+)(?=[,)])",
        lambda m: (
            f"mut {m.group(1)}: Int32" if m.group(1) in assigned
            else f"{m.group(1)}: Int32"),
        out)
    out = re.sub(r"\buint32_t (\w+)(?=[,)])", r"\1: Uint32", out)
    out = re.sub(r"\bvoid (\w+)(?=[,)])", r"\1: ()", out)
    # The C's array parameters and locals.
    out = re.sub(r"\bint32_t (\w+)\[\]", r"\1: &[Int32; 1]", out)
    out = re.sub(r"\bconst int32_t (\w+)\[\]", r"\1: &[Int32; 1]", out)
    out = re.sub(r"\bint32_t (\w+)\[\]\s*=", r"\1: &[Int32; 1] =", out)
    out = re.sub(r"\bf64 (\w+)\[\]\s*=", r"\1: &[f64; 1] =", out)
    out = re.sub(r"\bdouble (\w+)\[\]\s*=", r"\1: &[f64; 1] =", out)
    # The C's arrays that are of a size of their own.
    # The C's arrays, with their sizes and their kinds.
    out = out.replace("tx: &[f64; 1]", "tx: &[f64; 3]")
    out = out.replace("y: &[f64; 1]", "y: &[f64; 2]")
    out = out.replace("f: &[f64; 1]", "f: &[f64; 20]")
    out = out.replace("fq: &[f64; 1]", "fq: &[f64; 20]")
    out = out.replace("q: &[f64; 1]", "q: &[f64; 20]")
    out = out.replace("iq: &[Int32; 1]", "iq: &[Int32; 20]")
    out = out.replace("x: &mut [f64; 2], y: &mut [f64; 2]",
                      "x: &[f64; 3], y: &mut [f64; 2]")
    # The C's `y` holds the remainder, and the C's own kernel writes
    # three of them, though it writes the third only in a case the port
    # never takes. The port's `y` is three long, for Rust will not take
    # a write that is out of range.
    out = out.replace("y: &mut [f64; 2]", "y: &mut [f64; 3]")
    out = out.replace("let mut y: [f64; 2] = [0.0; 2];",
                      "let mut y: [f64; 3] = [0.0; 3];")
    # The C's return type where it is written before the name.
    out = re.sub(r"^int (\w+)\s*\(", r"fn \1(", out, flags=re.M)
    out = re.sub(r"^void (\w+)\s*\(", r"fn \1(", out, flags=re.M)

    # The local declarations, which the C writes as `type name` and which
    # Rust writes the same way but with its own types.
    out = re.sub(r"\bstatic const double\b", "const", out)
    out = re.sub(r"\bstatic const Int32\b", "const", out)
    out = re.sub(r"\bstatic Int32\b", "let", out)
    # The C's array declarations.
    out = re.sub(
        r"\bconst (f64|Int32|Uint32) (\w+)\[\]\s*=\s*\{",
        r"const \2: &[\1; 0] = &[", out)
    out = re.sub(r"\bstatic const Int32\b", "const", out)
    out = re.sub(r"\bstatic const Uint32\b", "const", out)
    out = re.sub(r"\bstatic double\b", "let", out)
    out = re.sub(r"\bconst double\b", "const", out)
    out = re.sub(r"\bdouble\b(?=\s+\w+\s*[=;,\[])", "f64", out)
    out = re.sub(r"\bint32_t\b(?=\s+\w+\s*[=;,\[])", "Int32", out)
    out = re.sub(r"\buint32_t\b(?=\s+\w+\s*[=;,\[])", "Uint32", out)
    out = re.sub(r"\bint\b(?=\s+\w+\s*[=;,\[])", "Int32", out)

    # The C's `x--` and `x++` as statements, which Rust writes the other
    # way round.
    out = re.sub(r"\b(\w+)\+\+;", r"\1 += 1;", out)
    out = re.sub(r"\b(\w+)--;", r"\1 -= 1;", out)
    # The C's `x++` and `x--` inside a larger expression, which Rust
    # writes as a statement before the expression.
    out = re.sub(r"\b(\w+)\+\+", r"\1 + 1", out)
    out = re.sub(r"\b(\w+)--", r"\1 - 1", out)

    # The C's ternary, which Rust writes as an `if`.
    # The C's ternary, which Rust writes as an `if`. The `if` is marked so
    # that the brace pass, which runs later, leaves it alone.
    out = re.sub(
        r"\(([^()]*;[^()]*;[^()]*)\)\s*\?\s*([^:;]+?)\s*:\s*([^;,]+?)\s*(?=[;,)])",
        lambda m: m.group(0),
        out)

    # The C's bare ternary, which is run after the C++'s own words are
    # Rust's, so that the branches are read as they will be.
    out = translate_ternary(out)

    # The C's `if` without braces, which Rust has no word for. The
    # statement that follows is found first, so that the brace may be
    # closed at its end.
    out = add_braces_to_bare_ifs(out)

    # The C's `lx | NegateWithWraparound<int32_t>(lx)`, which is a word:
    # the negation gives a signed word whose bits are the same, and the
    # `|` is on the word. The two words are the same one, so the text is
    # read and written with plain string handling, which is clearer than
    # a pattern for it.
    for word in ("ix", "iy", "hx", "hy"):
        for low in ("lx", "ly"):
            before = f"({low} | negate_with_wraparound({low})) >> 31"
            after = (
                f"(({low} as u32) | "
                f"(negate_with_wraparound({low} as i32) as u32)) >> 31"
            )
            out = out.replace(before, after)


    # The C's usual arithmetic conversions, which turn the signed word
    # into the unsigned one where the two meet. The C's low words are
    # unsigned, and the C's own words are signed, so a `|` or a `&` that
    # has one of each is on the unsigned word, and the port writes the
    # conversion out.
    def cast_low_mixes(text):
        """The C's `|` and `&` of a word with a word of the other kind.

        The two sides of the operator are found by counting the
        brackets, for a pattern cannot tell where a group ends when the
        group holds a group of its own. The `&&` and the `||` are the
        C's own choices and not the C's words, so they are left alone.
        """
        pattern = re.compile(r"(?<![&|])([&|])(?![&|]) +")
        # The words the C holds unsigned, and the words the C always
        # holds unsigned.
        unsigned = {
            name for name, kind in types.items() if kind == "Uint32"
        }
        unsigned |= {"low", "lx", "ly"}

        def is_unsigned(part):
            if "as u32" in part:
                return True
            return any(
                re.search(r"\b" + name + r"\b", part)
                for name in unsigned)

        def left_operand(text, end):
            """The expression on the left of the operator at `end`.

            A cast at the end of it is a part of it, for the C's cast is
            on the whole of the expression.
            """
            j = end - 1
            while j >= 0 and text[j] in " \t":
                j -= 1
            cast = re.search(
                r"\bas (?:u32|i32|Int32|Uint32|f64)$", text[:j + 1])
            if cast:
                _, begin = left_operand(text, cast.start())
                if begin < 0:
                    return None, -1
                return text[begin:j + 1], begin
            if j >= 0 and text[j] == ")":
                begin = group_start(text, j)
                if begin < 0:
                    return None, -1
                name = re.search(r"\w+$", text[:begin])
                if name:
                    begin = name.start()
                return text[begin:j + 1], begin
            word = re.search(r"\w+$", text[:j + 1])
            if not word:
                return None, -1
            return word.group(0), word.start()

        def group_start(text, at):
            """The bracket that opens the group that ends at `at`.

            The group is the one whose closing bracket is at `at`, and
            the brackets are counted to find the one that matches it.
            """
            depth = 0
            j = at
            while j >= 0:
                if text[j] == ")":
                    depth += 1
                elif text[j] == "(":
                    if depth == 0:
                        return -1
                    depth -= 1
                    if depth == 0:
                        return j
                j -= 1
            return -1

        def group_end(text, at):
            """The bracket that closes the group the text is in."""
            depth = 0
            j = at
            n = len(text)
            while j < n:
                if text[j] == "(":
                    depth += 1
                elif text[j] == ")":
                    if depth == 0:
                        return j
                    depth -= 1
                j += 1
            return -1

        operators = [found.group(1) for found in pattern.finditer(text)]

        # The operators are taken one at a time, from the last to the
        # first, for the group on the left of one operator may hold the
        # operator before it, and the text is read again for each.
        for index in range(len(operators) - 1, -1, -1):
            count = 0
            found = None
            for candidate in pattern.finditer(text):
                if count == index:
                    found = candidate
                    break
                count += 1
            if found is None:
                continue
            # The right side, which is a group or a name.
            start = found.end()
            if start < len(text) and text[start] == "(":
                end = group_end(text, start)
                if end < 0:
                    continue
                after = end + 1
            else:
                word = re.match(r"\w+", text[start:])
                if not word:
                    continue
                after = start + len(word.group(0))
            # The left side, which is a group or a name.
            left, begin = left_operand(text, found.start())
            if begin < 0:
                continue
            left = left.rstrip()
            right = text[found.end():after]
            if not (is_unsigned(left) or is_unsigned(right)):
                # The two sides are both signed, and the C's arithmetic on
                # them needs no conversion.
                continue
            text = (
                text[:begin]
                + f"({left} as u32) {found.group(1)} ({right} as u32)"
                + text[after:]
            )
        return text

    out = cast_low_mixes(out)

    # A hex word that does not fit in a signed word is an unsigned word
    # in the C, and the port writes it as one, for Rust will not take a
    # literal that is out of range.
    def word_suffix(match):
        literal = match.group(0)
        if int(literal, 16) >= 0x80000000:
            return literal + "u32"
        return literal

    out = re.sub(r"0x[0-9A-Fa-f]{8}(?![0-9A-Fa-fui])", word_suffix, out)

    # And a hex word that is added to a 64-bit word is a 64-bit word.
    for name, kind in types.items():
        if kind != "Uint64":
            continue
        out = re.sub(
            rf"\b{re.escape(name)} \+ (0x[0-9A-Fa-f]+)u32\b",
            lambda m: f"{name} + {m.group(1)}u64",
            out)


    # The C's `sign | (hx / 3 + B1)`, which is on the word, for the C
    # converts the signed `hx` to the unsigned one.

    # The index of an array, which is an arithmetic expression on names,
    # is made a `usize` as a whole. It is done here, so that the indices
    # in the `for` headers are made as well.
    out = re.sub(
        r"\b([A-Za-z_]\w*)\[([A-Za-z_]\w*(?:\s*[+\-]\s*[A-Za-z_]\w*)*)\]",
        r"\1[(\2) as usize]", out)

    # The C's calls that give the arrays by their names.
    out = re.sub(
        r"kernel_rem_pio2\(tx, y, e0, nx, 2, (\w+)\)",
        r"kernel_rem_pio2(&tx, y, e0, nx, 2, &\1)",
        out)
    out = out.replace(
        "ieee754_rem_pio2(x, y)", "ieee754_rem_pio2(x, &mut y)")

    # The C's `v = iy`, which gives a double the value of a word.
    out = re.sub(
        r"\b(v|q|x|f|fq|r|s|t) = (iy|ix|jz|jx|jv|jp|jk|n|m|q0|ih);",
        r"\1 = (\2) as f64;",
        out)

    # The C's `iq[jz] = (int32_t)z`, which gives a word the value of a
    # double.
    out = re.sub(
        r"\b(iq\[\([^\]]+\) as usize\]) = ([^;]+);",
        r"\1 = (\2) as Int32;",
        out)

    # The C's `f[jx + i] = ipio2[jv + i]`, which gives a double the
    # value of a word.
    out = re.sub(
        r"\b(f\[\([^\]]+\) as usize\]) = (ipio2\[\([^\]]+\) as usize\])",
        r"\1 = (\2) as f64",
        out)

    # The C's `goto` and its label, which Rust writes as a labelled
    # loop.
    out = translate_gotos(out)

    # The C's `switch`, which Rust writes as a `match`.
    out = translate_switches(out)

    # The C's `for` loop, which is Rust's `loop` with the C's own three
    # parts: the initialisation, the test, and the step. The port writes
    # it as it is, because the C's loops here count up and down and carry
    # more than one index, and a `for` over a range cannot say all of
    # that. It is done after the braces are added, so that the brace pass
    # does not see it.
    out = translate_for_loops(out)

    # The C's names that are Rust's keywords, which are given a prefix so
    # that they may be used. The C's own keywords have already been
    # translated by this point, so only the names that are left are
    # changed.
    for keyword in [
        "fn", "let", "const", "static", "mut", "ref", "type", "loop", "box",
        "impl", "trait", "enum", "struct", "match", "move", "unsafe", "mod",
        "pub", "use", "where", "as", "in", "for", "if", "else", "while",
        "self", "super", "crate", "true", "false",
    ]:
        # A name being used: a word followed by something that shows it
        # is a name and not a keyword of Rust's.
        out = re.sub(
            r"\b" + keyword + r"\b(?=\s*[:;,=\)\]])", "c_" + keyword, out)
        out = re.sub(
            r"\b" + keyword + r"\b(?=\s*[-+*/&|^])", "c_" + keyword, out)

    return out


# The functions that give a number back, and the type they give.


# The functions the JavaScript has, and that the port therefore has as
# its own, and the JavaScript's name for each.
PUBLIC = {
    "sin": "Math.sin",
    "cos": "Math.cos",
    "tan": "Math.tan",
    "asin": "Math.asin",
    "acos": "Math.acos",
    "atan": "Math.atan",
    "atan2": "Math.atan2",
    "exp": "Math.exp",
    "log": "Math.log",
    "log2": "Math.log2",
    "log10": "Math.log10",
    "log1p": "Math.log1p",
    "pow": "Math.pow",
    "cbrt": "Math.cbrt",
    "expm1": "Math.expm1",
    "sinh": "Math.sinh",
    "cosh": "Math.cosh",
    "tanh": "Math.tanh",
    "asinh": "Math.asinh",
    "acosh": "Math.acosh",
    "atanh": "Math.atanh",
}

RETURNING = {
    "__ieee754_rem_pio2": "Int32",
    "ieee754_rem_pio2": "Int32",
    "kernel_rem_pio2": "Int32",
    "kernel_cos": "f64",
    "kernel_sin": "f64",
    "kernel_tan": "f64",
    "__kernel_rem_pio2": "Int32",
    "__kernel_cos": "f64",
    "__kernel_sin": "f64",
    "__kernel_tan": "f64",
    "cos": "f64",
    "sin": "f64",
    "tan": "f64",
    "acos": "f64",
    "asin": "f64",
    "atan": "f64",
    "atan2": "f64",
    "exp": "f64",
    "log": "f64",
    "log2": "f64",
    "log10": "f64",
    "log1p": "f64",
    "pow": "f64",
    "cbrt": "f64",
    "expm1": "f64",
    "sinh": "f64",
    "cosh": "f64",
    "tanh": "f64",
    "asinh": "f64",
    "acosh": "f64",
    "atanh": "f64",
    "k_log1p": "f64",
}

# The functions to carry, in the order they are written.
FUNCTIONS = [
    "k_log1p",
    "__ieee754_rem_pio2",
    "__kernel_rem_pio2",
    "__kernel_cos",
    "__kernel_sin",
    "__kernel_tan",
    "cos",
    "sin",
    "tan",
    "acos",
    "asin",
    "atan",
    "atan2",
    "exp",
    "log",
    "log2",
    "log10",
    "log1p",
    "pow",
    "cbrt",
    "expm1",
    "sinh",
    "cosh",
    "tanh",
    "asinh",
    "acosh",
    "atanh",
]









def emit_tables(tables):
    """The constants and the tables, as Rust.

    A name that two packages share is given a number after it, so that the
    two do not collide: the C has them in different functions, and the
    port has them in one module.
    """
    out = []
    seen = {}
    for name in sorted(tables):
        value = tables[name]
        count = seen.get(name, 0)
        seen[name] = count + 1
        suffix = "" if count == 0 else f"_{count}"
        if isinstance(value, list):
            values = ", ".join(rust_double(v) for v in value)
            out.append(
                f"const {name}{suffix}: [f64; {len(value)}] = [{values}];")
        else:
            out.append(
                f"const {name}{suffix}: f64 = {rust_double(value)};")
    return "\n".join(out)


def find_grouped_constants(text):
    """The `static const double a = 1, b = 2;` written inside a function."""
    found = {}
    pattern = re.compile(
        r"static const (double|int32_t|uint32_t)\s+([^;]+);", re.S)
    for match in pattern.finditer(text):
        kind = match.group(1)
        for part in match.group(2).split(","):
            single = re.match(
                r"\s*(\w+)\s*=\s*([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)\s*$",
                part)
            if single:
                found[single.group(1)] = (kind, single.group(2))
    return found


def find_local_constants(text):
    """The `static const double name = value;` written inside a function."""
    found = {}
    pattern = re.compile(
        r"static const (double|int32_t|uint32_t)\s+(\w+)\s*=\s*"
        r"([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)\s*;")
    for match in pattern.finditer(text):
        found[match.group(2)] = (match.group(1), match.group(3))
    return found


def find_grouped_local_tables(text):
    """The C's grouped local tables, which are one type and several tables.

    The C writes `static const double bp[] = {...}, dp_h[] = {...};` and
    the port takes each table out and gives it its own name.
    """
    found = {}
    pattern = re.compile(
        r"static const (double|int32_t|uint32_t|int)\s+([^;]+);", re.S)
    for match in pattern.finditer(text):
        kind = match.group(1)
        body = match.group(2)
        # The tables, which are separated by commas, and whose values
        # have commas of their own.
        for single in re.finditer(
                r"(\w+)\s*\[\]\s*=\s*\{([^}]*)\}", body):
            values = re.findall(
                r"[-+]?(?:0[xX][0-9A-Fa-f]+|\d+(?:\.\d*)?(?:[eE][-+]?\d+)?)",
                single.group(2))
            found[single.group(1)] = (kind, values)
    return found


def find_local_tables(text):
    """The `static const` tables that are written inside a function.

    Rust wants them at the top of the module, so they are taken out of
    the function and given their real size, which the C leaves to the
    initialiser.
    """
    found = {}
    pattern = re.compile(
        r"static const (double|int32_t|uint32_t|int)\s+(\w+)\[(?:[^\]]*)\]"
        r"\s*=\s*\{(.*?)\};",
        re.S)
    for match in pattern.finditer(text):
        kind = match.group(1)
        name = match.group(2)
        body = match.group(3)
        if kind not in ("double", "int32_t", "uint32_t", "int"):
            continue
        # The C's values are the whole words of a number, in decimal or
        # in hex, and the hex ones are kept as they are.
        values = re.findall(
            r"[-+]?(?:0[xX][0-9A-Fa-f]+|\d+(?:\.\d*)?(?:[eE][-+]?\d+)?)",
            body)
        found[name] = (kind, values)
    return found


def main():
    if not SOURCE.is_file():
        print(f"the source is not there: {SOURCE}", file=sys.stderr)
        return 1
    text, original = strip_comments(SOURCE.read_text())
    tables = find_tables(text)
    local = find_local_tables(text)
    # The same tables, in the text where the types have been renamed.
    for name, (kind, values) in find_local_tables(
            text.replace("int32_t", "Int32").replace("uint32_t", "Uint32")
        ).items():
        local.setdefault(name, (kind, values))
    for name, (kind, values) in find_grouped_local_tables(text).items():
        if name not in local:
            local[name] = (kind, values)
    local_constants = find_local_constants(text)
    local_constants.update(find_grouped_constants(text))

    pieces = [HEADER]
    missing = []
    bodies = []
    for name in FUNCTIONS:
        body = find_body(text, name)
        if body is None:
            missing.append(name)
            continue
        # The local declarations are taken out first, while they are
        # still in the C's own words; they are at the top of the module.
        body = re.sub(
            r"\s*static\s+(?:const\s+)?(?:volatile\s+)?"
            r"(double|int32_t|uint32_t|int)\s+"
            r"([^;]+);",
            lambda match: take_group(match, name.lstrip("_")),
            body, flags=re.S)
        for table_name in local:
            body = re.sub(
                r"\s*(?:static const )?(double|int32_t|uint32_t|int|Int32|Uint32)"
                r"\s+" + table_name + r"\[\]\s*=\s*\{.*?\}(?:,|;)",
                "", body, flags=re.S)
        bodies.append((name.lstrip("_"), translate_body(body, tables, name)))

    # Every constant the port writes out is in one list, and a name that
    # two of them share is given the place it came from after it, so that
    # the two do not collide.
    # The tables' values, so that an alias of one of their entries may be
    # written out with the value the entry holds.
    table_values = {}
    for name, value in tables.items():
        if isinstance(value, list):
            table_values.setdefault(name, value)
    for name, (kind, values) in local.items():
        table_values.setdefault(name, values)
    for name, kind, value, where in STATIC_NAMES:
        if isinstance(value, tuple) and value[0] == "array":
            table_values.setdefault(name, value[1])

    everything = []
    seen = set()
    for name, value in tables.items():
        everything.append((name, "double", value, ""))
        seen.add((name, ""))
    for name, (kind, value) in local_constants.items():
        if (name, "") in seen:
            continue
        everything.append((name, kind, value, ""))
        seen.add((name, ""))
    for name, (kind, values) in local.items():
        if (name, "") in seen:
            continue
        everything.append((name, kind, ("array", values), ""))
        seen.add((name, ""))
    for name, kind, value, where in STATIC_NAMES:
        # A `#define` is the function's own, and it is kept apart from a
        # constant of the same name that the function holds.
        if where and isinstance(value, tuple) and value[0] == "table":
            key = (name, where, "define")
        else:
            key = (name, where)
        if key in seen:
            continue
        everything.append((name, kind, value, where))
        seen.add(key)

    counts = {}
    for name, kind, value, where in everything:
        counts[name] = counts.get(name, 0) + 1
    # The places that share a name are numbered, so that the two do not
    # collide. The number is worked out once, and both the constant and
    # the code that uses it are given it, so that the two agree.
    running = {}
    for name, kind, value, where in everything:
        key = (name, where)
        running[key] = running.get(key, 0) + 1

    # The tables of a place, so that an entry written in a function is
    # read from the table of that function.
    place_values = {}
    for name, kind, value, where in everything:
        if isinstance(value, tuple) and value[0] == "array":
            place_values.setdefault((name, where), value[1])

    # The name each constant is given, worked out here and used both for
    # the constant and for the code that uses it, so that the two agree.
    given = []
    for name, kind, value, where in everything:
        suffix = ""
        if counts[name] > 1:
            key = (name, where)
            number = running[key]
            suffix = f"_{where.upper()}_{number}"
        given.append((name, kind, value, where, suffix))
    # The name each table is given, so that an alias of one of its entries
    # may name it as it is written out. The name is worked out with the
    # same rule the constants use.
    table_names = {}
    for name, kind, value, where, suffix in given:
        if isinstance(value, tuple) and value[0] == "array":
            table_names.setdefault((name, where), []).append(name + suffix)

    for name, kind, value, where, suffix in given:
        rust_type = {"double": "f64", "int32_t": "Int32", "uint32_t": "Int32",
                     "int": "Int32"}[kind]
        if isinstance(value, tuple) and value[0] == "table":
            # An entry of a table, which the port writes out as a
            # constant of its own, with the value the entry holds.
            table, index = value[1], value[2]
            entry = place_values.get((table, where))
            if entry is None:
                entry = table_values.get(table)
            if entry is not None and index < len(entry):
                pieces.append(
                    f"const {name}{suffix}: f64 = {rust_double(entry[index])};")
            else:
                names = table_names.get((table, where), [])
                target = names[0] if names else table
                pieces.append(
                    f"const {name}{suffix}: f64 = {target.upper()}[{index}];")
        elif isinstance(value, tuple) and value[0] == "alias":
            # The alias of a table, which is the table itself. The code
            # uses it as a table, so it is written out as one.
            table = value[1]
            names = table_names.get((table, where), [])
            target = names[0] if names else table
            entry = table_values.get(table)
            if entry is None:
                pieces.append(
                    f"const {name}{suffix}: f64 = {target.upper()};")
            else:
                values = ", ".join(rust_double(v) for v in entry)
                pieces.append(
                    f"const {name}{suffix}: [f64; {len(entry)}] = [{values}];")
        elif isinstance(value, tuple) and value[0] == "table":
            pass
        elif isinstance(value, tuple) and value[0] == "array":
            values = ", ".join(
                rust_number(v, kind) for v in value[1])
            pieces.append(
                f"const {name}{suffix}: [{rust_type}; {len(value[1])}] = [{values}];")
        elif isinstance(value, list):
            values = ", ".join(rust_double(v) for v in value)
            pieces.append(
                f"const {name}{suffix}: [{rust_type}; {len(value)}] = [{values}];")
        else:
            pieces.append(f"const {name}{suffix}: {rust_type} = {value};")
    pieces.append("")

    # The code, with the names that two places share told apart. The
    # number the constant was given is the number the code is given too.
    for name, rust in bodies:
        for other, kind, value, where, suffix in given:
            if where != name and where != name.lstrip("_"):
                continue
            if counts[other] <= 1:
                continue
            rust = re.sub(
                r"\b" + re.escape(other) + r"\b",
                other + suffix, rust)
        # The C's `k_log1p` uses the file's own constants, which the
        # port has named after the file and not after the function.
        for index in range(1, 8):
            rust = rust.replace(f"Lg{index}", f"Lg{index}_LOG_1")
            rust = rust.replace(f"Lg{index}_LOG_1_LOG_1", f"Lg{index}_LOG_1")

        rust = rust.replace("__ieee754_rem_pio2", "ieee754_rem_pio2")
        rust = rust.replace("__kernel_rem_pio2", "kernel_rem_pio2")
        rust = rust.replace("__kernel_cos", "kernel_cos")
        rust = rust.replace("__kernel_sin", "kernel_sin")
        rust = rust.replace("__kernel_tan", "kernel_tan")
        # The function gives a number back, which the C's signature says.
        kind = RETURNING.get(name)
        if kind:
            rust = re.sub(
                r"^((?:#\[inline\]\s*)?fn " + re.escape(name) +
                r"\([^)]*\))\s*\{",
                rf"\1 -> {kind} {{",
                rust, flags=re.M)
        # The C's `fw * iq[i]`, which is a double times a word.
        rust = re.sub(
            r"\b(fw|q|x|f|fq) \* (iq\[\([^\]]+\) as usize\])",
            r"\1 * (\2) as f64",
            rust)
        if name in PUBLIC:
            # The function is the JavaScript's, and the port has it as
            # its own.
            rust = re.sub(
                r"^(?:#\[inline\]\s*)?fn " + re.escape(name) + r"\b",
                "pub fn " + name,
                rust, flags=re.M)
            pieces.append(
                f"/// The JavaScript's `{PUBLIC[name]}`, as V8 has it.")
        else:
            pieces.append("/// The C's function, as it is here.")
        pieces.append(rust)
        pieces.append("")

    if missing:
        print(f"not found: {missing}", file=sys.stderr)
    TARGET.write_text("\n".join(pieces))
    print(f"wrote {TARGET} ({len('\n'.join(pieces).splitlines())} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
