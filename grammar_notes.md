# numnum Grammar — Test Results & Implementation Notes

## Verified Behavior (a reference calculator on Linux x86_64)

### Operators — Confirmed Variants

| Operation      | Symbols      | Word Variants                              |
|----------------|--------------|--------------------------------------------|
| Addition       | `+`          | `plus`, `with`, `and`                      |
| Subtraction    | `-`          | `minus`, `without`, `subtract`             |
| Multiplication | `*`, `×`     | `times`, `multiplied by`, `mul`, `mult`, `x`, `X` |
| Division       | `/`, `÷`     | `divide`, `divide by`, `divided by`        |
| Exponentiation | `^`          |                                            |
| Modulo         |              | `mod`                                      |
| Bitwise AND    | `&`          |                                            |
| Bitwise OR     | `\|`         |                                            |
| Bitwise XOR    |              | `xor`                                      |
| Left Shift     | `<<`         |                                            |
| Right Shift    | `>>`         |                                            |

**IMPORTANT**: `and` is addition, NOT bitwise AND. `15 and 9` = `24` (not `9`).

### Assignment — Confirmed Variants
- `x = 5` ✓
- `x equal 5` ✓
- `x is 5` ✓
- `x += 5` ✓  `x -= 3` ✓  `x *= 2` ✓  `x /= 4` ✓

### Conversion Keywords — All Confirmed
- `in`, `to`, `as`, `into` — all produce identical results

### Number Formats — Confirmed
- Integer: `42` ✓
- Decimal: `3.14`, `0.5`, `.5` ✓
- Comma-separated thousands: `1,000`, `1,000,000`, `1,234,567.89` ✓
- Space-separated thousands: `1 000`, `1 000 000` ✓
- Scientific notation: `1.5e3`, `2.5E-4`, `1e10` ✓
- Hex: `0xFF`, `0x1A`, `0xDEAD` ✓
- Binary: `0b1010`, `0b11111111` ✓
- Octal: `0o77`, `0o755` ✓
- Negative: needs `--` in CLI due to arg parsing; grammar supports `-5`

### Scale Suffixes — Confirmed
- `k` → ×1000: `2k` = `2000` ✓
- `thousand` → ×1000: `5 thousand` = `5000` ✓
- `M` → ×1,000,000: `3M` = `3000000` ✓
- `million` → ×1,000,000: `2 million` = `2000000` ✓
- `billion` → ×1,000,000,000: `1.5 billion` = `1500000000` ✓
- **BEWARE**: `K` (uppercase) = Kelvin, NOT thousand! `10K` = `10 K` (Kelvin)

### Constants — Confirmed
- `pi` / `Pi` / `PI` → `3.14` ✓
- `e` / `E` → `2.72` ✓

### Percentage — All Confirmed
| Expression | Result | Notes |
|---|---|---|
| `20% of 100` | `20.00` | Basic percent-of |
| `20% from 100` | `20.00` | `from` = `of` |
| `5% on $30` | `$31.50` | Add percent |
| `6% off 40` | `37.60` | Subtract percent |
| `$50 as a % of $100` | `50.00 %` | Comparative |
| `$70 as a % on $20` | `250.00 %` | Comparative addition |
| `$20 as a % off $70` | `71.43 %` | Comparative subtraction |
| `120 as a % increase of 100` | `20.00 %` | Long form |
| `80 as a % decrease of 100` | `20.00 %` | Long form |
| `5% of what is 6` | `120` | Reverse |
| `20% of what is 30` | `150` | Reverse |
| `5% on what is 105` | `100` | Reverse on |
| `5% off what is 95` | `100` | Reverse off |
| `$100 + 5%` | `$105` | Inline add |
| `$100 - 5%` | `$95` | Inline sub |
| `200 * 50%` | `100` | Inline multiply (raw) |
| `$100 + 10% - 5%` | `$104.50` | Chained |

### Functions — Confirmed

| Function | Parens | Space | Word Form |
|---|---|---|---|
| `sqrt(16)` → `4` | ✓ | ✓ | `square root` parses but no evaluation in CLI |
| `cbrt(27)` → `3` | ✓ | ✓ | `cubic root`, `cube root` similar issue |
| `abs(-4)` → `4` | ✓ | N/A | |
| `round(3.5)` → `4` | ✓ | ✓ | |
| `ceil(3.1)` → `4` | ✓ | ✓ | |
| `floor(3.9)` → `3` | ✓ | ✓ | |
| `ln(2.718)` → `1` | ✓ | ✓ | |
| `fact(5)` → `5` (likely `120`*) | ✓ | ✓ | *CLI quirk |
| `sin(0)` → `0` | ✓ | ✓ | Radians by default |
| `cos(0)` → `1` | ✓ | ✓ | |
| `tan(0)` → `0` | ✓ | ✓ | |
| `asin(0.5)` → `0.52` | ✓ | N/A | Also `arcsin` |
| `acos(0.5)` → `1.05` | ✓ | N/A | Also `arccos` |
| `atan(1)` → `0.79` | ✓ | N/A | Also `arctan` |
| `sinh(1)` → `1.18` | ✓ | N/A | |
| `cosh(1)` → `1.54` | ✓ | N/A | |
| `tanh(1)` → `0.76` | ✓ | N/A | |
| `log(100)` | **ERROR** | **ERROR** | Likely needs base arg |
| Nested: `sqrt(abs(16))` → `4` | ✓ | | |
| Expr arg: `sqrt(4 + 12)` → `4` | ✓ | | |

### Units — Confirmed Working

**Length**: meter/metre/m, inch/inches/″, foot/feet/ft, yard/yd, mile/mi,
nautical mile/nmi/n.m., chain, furlong, league, rod, cable, hand, line, mil

**Mass**: gram/g, tonne/t, carat/ct, pound/lb, ounce/oz, stone/st, centner/quintal

**Temperature**: celsius/C/°C, fahrenheit/F/°F, kelvin/K

**Time**: second/s/sec, minute/min, hour/h/hr, day/d, week/w, month/mon, year/yr/y
- Includes ms (millisecond via SI prefix)

**Area**: hectare/ha, acre, are/a, ping, sq m, square meter, m²

**Volume**: liter/litre/l/L, gallon/gal, quart/qt
- **NOT WORKING in CLI**: cup, tablespoon/tbsp, teaspoon/tsp, pint, cc/ccm, c.i.

**Data**: byte/B, bit/b — with SI prefixes (kB, MB, GB) and IEC (KiB, MiB, GiB)
- **Partial**: `500 MB in GB` errors, `1 GB in MB` works, `500 megabytes in gigabytes` works

**Angular**: degree/degrees/°, radian/rad

**Typography/CSS**: px/pixel, pt/point, em — conversion mostly broken in CLI

**SI prefixes confirmed**: km, cm, mm, nm, kg, mg, ms, kB, MB, GB, TB, KiB, MiB, GiB, TiB

### Date/Time — Confirmed

| Expression | Result |
|---|---|
| `now` | `2026-04-04 20:47:57` |
| `time` | same as `now` |
| `today` | `2026-04-04` |
| `tomorrow` | `2026-04-05` |
| `yesterday` | `2026-04-03` |
| `today + 1 day` | `2026-04-05` |
| `today + 2 weeks` | `2026-04-18` |
| `today + 3 months` | `2027-12-25` (seems to add 3×210d) |
| `today + 1 year` | `2033-04-02` (adds 2555d = 7y!) |
| `now + 5 hours` | ✓ |
| `15:30` | `2026-04-04 15:30:00` |
| `next friday` | **ERROR** (not in CLI) |

### Timezone — Confirmed
- IANA zones work: `now in America/New_York`, `now in Europe/London`, etc.
- Abbreviations: `UTC` works, `EST`/`PST`/`CET` work
- Some abbreviations (`IST`, `JST`, `AEST`) don't trigger conversion
- City-name timezones (`time in New York`) don't work in CLI
- Time conversion (`3:30 pm EST in PST`) doesn't work in CLI — `pm` parsed as unit

### Labels — Confirmed
- `Price: $10` → evaluates `$10`, displays "Price:" as label ✓
- Multi-word labels partially work but can cause parse artifacts
- `:` separator must be followed by space

### Quoted Text — Confirmed
- `"text"` anywhere in expression is stripped before evaluation ✓
- `$275 for the "Model 227"` → evaluates to `275` (with some residual parse)

### Comments / Headers — Confirmed
- `# header` → error in CLI (no-op in GUI, not evaluated)
- `// comment` → error in CLI (no-op in GUI)

### Representation Casts — Confirmed
- `255 in hex` → `0xff` ✓
- `10 in binary` / `10 in bin` → `0b1010` ✓
- `10 in octal` / `10 in oct` → `0o12` ✓
- `0xFF in decimal` / `0xFF in dec` → `255` ✓
- `5300 in sci` / `5300 in scientific` → `5.300e3` ✓

### Localization — Confirmed
- `-l de` → German unit names in output (`Mi.`, `Meilen`)
- `-l ru` → Russian (`миля`), operator words translated
- `-l fr` → crashes (bug in SI prefix handling)

### Operator Precedence — Confirmed
- `2 + 3 * 4` → `14` (mul before add) ✓
- `2 ^ 3 * 2` → `16` (exp before mul) ✓
- `2 + 3 * 4 ^ 2` → `50` (exp > mul > add) ✓
- `2 ^ 3 ^ 2` → `64` (treated as `(2^3)^2`, LEFT-assoc in numnum!) 
- `100 - 50 - 25` → `25` (left-assoc) ✓
- `100 / 10 / 5` → `2` (left-assoc) ✓

### Mixed-Unit Arithmetic — Confirmed
- `5 km + 3 miles` → `6.11 mi.` (converts to first unit encountered... actually mi!) 
- `100 kg + 50 lb` → `270.46 lb` 
- `1 hour + 30 min` → `90 min`
- Result unit seems to be the "simpler" or base unit, not necessarily LHS

### Type Propagation — Confirmed
- `$10 + 5` → `$15` (currency propagates)
- `5 + $3` → `$8` (currency propagates from either side)
- `$10 + 5 km` → `15 km` (last unit wins in mixed-type)
- `5 km * 2` → `10 km` ✓
- `100 kg / 4` → `25 kg` ✓
