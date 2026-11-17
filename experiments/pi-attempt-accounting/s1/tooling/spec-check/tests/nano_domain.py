"""Nano domain shared by the schema regeneration and the schema tests.

`spec/ck.json` MONEY defines Nano as checked i128 and SERIALIZATION defines nonnegative canonical
decimal strings. The standard-schema pattern below accepts exactly the canonical decimal strings
0..=i128::MAX: no sign, no leading zero, no exponent.
"""
I128_MAX = (1 << 127) - 1
OLD_PATTERN = "^(0|[1-9][0-9]*)$"


def nano_pattern(maximum=I128_MAX):
    top = str(maximum)
    width = len(top)
    free = lambda n: "" if n == 0 else "[0-9]" if n == 1 else "[0-9]{%d}" % n
    alternatives = ["0", "[1-9]" + ("[0-9]{0,%d}" % (width - 2))]
    for position in range(width):
        digit = int(top[position])
        low = 1 if position == 0 else 0
        if digit > low:
            cls = str(low) if digit - 1 == low else "[%d-%d]" % (low, digit - 1)
            alternatives.append(top[:position] + cls + free(width - position - 1))
    alternatives.append(top)
    return "^(" + "|".join(alternatives) + ")$"


if __name__ == "__main__":
    print(nano_pattern())
