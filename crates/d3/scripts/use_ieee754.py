"""The shape and path modules' own trigonometry, taken from the port.

The JavaScript these modules are a port of runs on V8, and V8's
`Math.cos`, `Math.sin` and the rest are V8's own copy of fdlibm, and not
the platform's. The two differ by an ulp on some of the values, and a
path's last digit is that ulp. So the modules call the port's own
functions, which are a translation of V8's.

    python3 scripts/use_ieee754.py
"""

import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent

# The methods, and the port's own function for each.
FUNCTIONS = {
    "cos": "cos",
    "sin": "sin",
    "tan": "tan",
    "asin": "asin",
    "acos": "acos",
    "atan2": "atan2",
    "exp": "exp",
    "ln": "log",
    "cbrt": "cbrt",
    "sinh": "sinh",
    "cosh": "cosh",
    "tanh": "tanh",
    "asinh": "asinh",
    "acosh": "acosh",
    "atanh": "atanh",
    "log2": "log2",
    "log10": "log10",
    "log1p": "log1p",
    "expm1": "expm1",
}


def receiver_start(text, at):
    """Where the expression that a method is called on begins."""
    i = at
    while i > 0 and text[i - 1] in " \t\n":
        i -= 1
    if i == 0:
        return None
    character = text[i - 1]
    if character in ")]":
        depth = 0
        j = i - 1
        while j >= 0:
            if text[j] in ")]":
                depth += 1
            elif text[j] in "([":
                depth -= 1
                if depth == 0:
                    break
            j -= 1
        if j < 0:
            return None
        # The name in front of the group, which is a part of it.
        k = j
        while k > 0 and (text[k - 1].isalnum() or text[k - 1] == "_"):
            k -= 1
        return k
    if character.isalnum() or character in "_.":
        j = i - 1
        while j >= 0 and (text[j].isalnum() or text[j] in "_."):
            j -= 1
        return j + 1
    return None


def rewrite(text):
    """Every call of a method, as a call of the port's own function."""
    changed = True
    while changed:
        changed = False
        for method, function in FUNCTIONS.items():
            for match in re.finditer(r"\." + method + r"\(", text):
                at = match.start()
                start = receiver_start(text, at)
                if start is None:
                    continue
                receiver = text[start:at]
                if receiver.startswith("ieee754::"):
                    continue
                if method == "atan2":
                    # The two arguments, which are the receiver and the
                    # one in the brackets.
                    end = match.end()
                    depth = 1
                    while end < len(text) and depth:
                        if text[end] == "(":
                            depth += 1
                        elif text[end] == ")":
                            depth -= 1
                            if depth == 0:
                                break
                        end += 1
                    argument = text[match.end():end]
                    text = (
                        text[:start]
                        + f"ieee754::atan2({receiver}, {argument})"
                        + text[end + 1:]
                    )
                    changed = True
                    break
                text = (
                    text[:start]
                    + f"ieee754::{function}("
                    + receiver
                    + text[match.end():]
                )
                changed = True
                break
            if changed:
                break
    return text


def main():
    for name in ("src/path.rs", "src/shape/arc.rs", "src/shape/curve.rs",
                 "src/shape/line.rs", "src/shape/area.rs",
                 "src/shape/pie.rs", "src/shape/link.rs",
                 "src/shape/symbol.rs", "src/shape/stack.rs",
                 "src/shape/radial.rs"):
        path = ROOT / name
        if not path.exists():
            continue
        text = path.read_text()
        after = rewrite(text)
        if after == text:
            continue
        if "use crate::ieee754;" not in after:
            # The import goes after the module's own documentation.
            lines = after.split("\n")
            at = 0
            while at < len(lines) and (
                    lines[at].startswith("//!") or not lines[at].strip()):
                at += 1
            lines.insert(at, "use crate::ieee754;\n")
            after = "\n".join(lines)
        path.write_text(after)
        print("rewrote", name)


if __name__ == "__main__":
    main()
