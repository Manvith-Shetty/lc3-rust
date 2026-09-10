#!/usr/bin/env python3
"""Minimal LC-3 assembler: .asm in, big-endian .obj out.

Just enough to build the example programs in this directory. Two passes:
the first records label addresses, the second emits one big-endian word per
instruction (the object file's first word is the origin, which is what
main.rs reads before loading the rest into memory).

    python3 asm.py hello.asm hello.obj
"""

import re
import sys

TRAPS = {"GETC": 0x20, "OUT": 0x21, "PUTS": 0x22, "IN": 0x23,
         "PUTSP": 0x24, "HALT": 0x25}

BR_FLAGS = {"N": 4, "Z": 2, "P": 1}

ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\", '"': '"'}


def unescape(s):
    out, i = [], 0
    while i < len(s):
        if s[i] == "\\" and i + 1 < len(s):
            out.append(ESCAPES[s[i + 1]])
            i += 2
        else:
            out.append(s[i])
            i += 1
    return "".join(out)


def parse_lines(text):
    """-> [(label|None, mnemonic, [operands])] with comments/blanks dropped."""
    rows = []
    for raw in text.splitlines():
        # Strip comments, but not a ';' living inside a string literal.
        line, in_str = [], False
        for ch in raw:
            if ch == '"':
                in_str = not in_str
            if ch == ";" and not in_str:
                break
            line.append(ch)
        line = "".join(line).strip()
        if not line:
            continue

        # A .STRINGZ operand is taken whole; everything else splits on commas.
        m = re.match(r'^(.*?)\.STRINGZ\s+"(.*)"\s*$', line, re.IGNORECASE)
        if m:
            head, literal = m.group(1).strip(), m.group(2)
            rows.append((head or None, ".STRINGZ", [unescape(literal)]))
            continue

        tokens = line.split(None, 1)
        head = tokens[0]
        rest = tokens[1] if len(tokens) > 1 else ""
        label = None
        if not is_mnemonic(head):
            label = head.rstrip(":")
            tokens = rest.split(None, 1)
            if not tokens:
                rows.append((label, None, []))
                continue
            head, rest = tokens[0], tokens[1] if len(tokens) > 1 else ""
        operands = [o.strip() for o in rest.split(",")] if rest.strip() else []
        rows.append((label, head.upper(), operands))
    return rows


MNEMONICS = {"ADD", "AND", "NOT", "BR", "JMP", "RET", "JSR", "JSRR", "LD",
             "LDI", "LDR", "LEA", "ST", "STI", "STR", "TRAP", "RTI"}


def is_mnemonic(tok):
    t = tok.upper()
    if t in MNEMONICS or t in TRAPS or t.startswith("."):
        return True
    return t.startswith("BR") and all(c in "NZP" for c in t[2:])


def size_of(mnemonic, operands):
    if mnemonic is None:
        return 0
    if mnemonic == ".STRINGZ":
        return len(operands[0]) + 1
    if mnemonic == ".BLKW":
        return value(operands[0])
    if mnemonic in (".ORIG", ".END"):
        return 0
    return 1


def value(tok):
    """Immediate: #10, xFF, b1010, or a bare decimal."""
    t = tok.strip()
    if t[0] in "#":
        return int(t[1:], 10)
    if t[0] in "xX":
        return int(t[1:], 16)
    if t[0] in "bB":
        return int(t[1:], 2)
    return int(t, 10)


def reg(tok):
    return int(tok.strip()[1:])


def to_bits(n, width, what):
    lo, hi = -(1 << (width - 1)), (1 << width) - 1
    if not lo <= n <= hi:
        raise ValueError(f"{what} {n} does not fit in {width} bits")
    return n & ((1 << width) - 1)


def offset(target, pc, width, labels):
    addr = labels[target] if target in labels else value(target)
    return to_bits(addr - pc, width, f"offset to {target}")


def assemble(text):
    rows = parse_lines(text)

    origin, addr, labels = None, None, {}
    for label, mnemonic, operands in rows:
        if mnemonic == ".ORIG":
            origin = addr = value(operands[0])
            continue
        if origin is None:
            raise ValueError("code before .ORIG")
        if label:
            labels[label] = addr
        addr += size_of(mnemonic, operands)

    words, addr = [], origin
    for label, m, ops in rows:
        if m in (None, ".ORIG", ".END"):
            continue
        pc = addr + 1  # PC-relative offsets are taken from the *next* instruction

        if m == ".STRINGZ":
            words.extend(ord(c) for c in ops[0])
            words.append(0)
        elif m == ".FILL":
            v = labels[ops[0]] if ops[0] in labels else value(ops[0])
            words.append(v & 0xFFFF)
        elif m == ".BLKW":
            words.extend([0] * value(ops[0]))
        elif m in TRAPS:
            words.append(0xF000 | TRAPS[m])
        elif m == "TRAP":
            words.append(0xF000 | (value(ops[0]) & 0xFF))
        elif m in ("ADD", "AND"):
            op = 0x1000 if m == "ADD" else 0x5000
            w = op | (reg(ops[0]) << 9) | (reg(ops[1]) << 6)
            src = ops[2].strip()
            if src[0] in "rR" and src[1:].isdigit():
                words.append(w | reg(src))
            else:
                words.append(w | 0x20 | to_bits(value(src), 5, "imm5"))
        elif m == "NOT":
            words.append(0x903F | (reg(ops[0]) << 9) | (reg(ops[1]) << 6))
        elif m == "JMP":
            words.append(0xC000 | (reg(ops[0]) << 6))
        elif m == "RET":
            words.append(0xC1C0)
        elif m == "JSR":
            words.append(0x4800 | offset(ops[0], pc, 11, labels))
        elif m == "JSRR":
            words.append(0x4000 | (reg(ops[0]) << 6))
        elif m == "RTI":
            words.append(0x8000)
        elif m.startswith("BR"):
            flags = m[2:] or "NZP"
            n = sum(BR_FLAGS[c] for c in flags)
            words.append((n << 9) | offset(ops[0], pc, 9, labels))
        elif m in ("LD", "LDI", "LEA", "ST", "STI"):
            op = {"LD": 0x2000, "LDI": 0xA000, "LEA": 0xE000,
                  "ST": 0x3000, "STI": 0xB000}[m]
            words.append(op | (reg(ops[0]) << 9) | offset(ops[1], pc, 9, labels))
        elif m in ("LDR", "STR"):
            op = 0x6000 if m == "LDR" else 0x7000
            words.append(op | (reg(ops[0]) << 9) | (reg(ops[1]) << 6)
                         | to_bits(value(ops[2]), 6, "offset6"))
        else:
            raise ValueError(f"unsupported mnemonic: {m}")

        addr += size_of(m, ops)

    return origin, words


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: asm.py <input.asm> <output.obj>")
    with open(sys.argv[1]) as f:
        origin, words = assemble(f.read())
    with open(sys.argv[2], "wb") as f:
        f.write(origin.to_bytes(2, "big"))
        for w in words:
            f.write((w & 0xFFFF).to_bytes(2, "big"))
    print(f"{sys.argv[2]}: origin {origin:#06x}, {len(words)} words")


if __name__ == "__main__":
    main()
