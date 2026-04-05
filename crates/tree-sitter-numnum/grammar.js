/// <reference types="tree-sitter-cli/dsl" />

module.exports = grammar({
  name: "numnum",

  extras: $ => [/[ \t]/],

  rules: {
    document: $ => repeat($.line),

    line: $ => choice(
      $.comment,
      $.header,
      $.statement,
      $.blank_line,
    ),

    blank_line: $ => /\n/,

    comment: $ => seq("//", /.*/),

    header: $ => seq("#", /.*/),

    statement: $ => seq(
      optional($.label),
      $._expression,
    ),

    label: $ => seq(
      $.identifier,
      ":",
    ),

    _expression: $ => choice(
      $.assignment,
      $.compound_assignment,
      $.conversion,
      $.as_a_percent,
      $.binary_expression,
      $.inline_percent,
      $.percent_expression,
      $.function_call,
      $.unary_expression,
      $.parenthesized,
      $.number_with_unit,
      $.currency_value,
      $.scaled_number,
      $.aggregation,
      $.number,
      $.variable,
    ),

    assignment: $ => seq(
      field("name", $.identifier),
      choice("=", "equal", "is"),
      field("value", $._expression),
    ),

    compound_assignment: $ => seq(
      field("name", $.identifier),
      field("operator", choice("+=", "-=", "*=", "/=")),
      field("value", $._expression),
    ),

    // Operator precedence (matching our Pratt parser binding powers):
    //   conversion:   1
    //   as_a_percent: 3
    //   bitwise:      5
    //   shift:        7
    //   additive:     9
    //   multiplicative: 11
    //   power:        13 (left-assoc)
    //   unary:        15
    //   function:     17
    //   percent:      19

    binary_expression: $ => choice(
      // Additive
      prec.left(9, seq($._expression, field("operator", choice(
        "+", "-",
        "plus", "with", "and",
        "minus", "without", "subtract",
      )), $._expression)),
      // Multiplicative
      prec.left(11, seq($._expression, field("operator", choice(
        "*", "/", "\u00D7", "\u00F7",
        "times", "multiply", "mul", "mult", "multiplied by",
        "divide", "divide by", "divided by",
        "mod",
      )), $._expression)),
      // Power (left-assoc)
      prec.left(13, seq($._expression, field("operator", "^"), $._expression)),
      // Bitwise
      prec.left(5, seq($._expression, field("operator", choice("&", "|", "xor")), $._expression)),
      // Shift
      prec.left(7, seq($._expression, field("operator", choice("<<", ">>")), $._expression)),
    ),

    conversion: $ => prec.left(1, seq(
      $._expression,
      choice("in", "to", "as", "into"),
      field("target", choice($.unit_identifier, $.repr_keyword)),
    )),

    repr_keyword: $ => choice(
      "hex", "binary", "bin", "octal", "oct",
      "decimal", "dec", "scientific", "sci",
      "exp", "exponent", "exponential",
    ),

    inline_percent: $ => prec.left(9, seq(
      $._expression,
      field("operator", choice("+", "-")),
      $.percent_literal,
    )),

    as_a_percent: $ => prec.left(3, seq(
      $._expression,
      choice(
        "as a % of", "as a percent of",
        "as a % on", "as a percent on",
        "as a % increase of", "as a percent increase of",
        "as a % off", "as a percent off",
        "as a % decrease of", "as a percent decrease of",
      ),
      $._expression,
    )),

    percent_expression: $ => choice(
      $.percent_of,
      $.percent_on,
      $.percent_off,
      $.reverse_percent,
      $.percent_literal,
    ),

    percent_of: $ => prec(19, seq(
      $.percent_literal,
      choice("of", "from"),
      $._expression,
    )),

    percent_on: $ => prec(19, seq(
      $.percent_literal,
      "on",
      $._expression,
    )),

    percent_off: $ => prec(19, seq(
      $.percent_literal,
      "off",
      $._expression,
    )),

    reverse_percent: $ => prec(19, seq(
      $.percent_literal,
      choice("of what is", "on what is", "off what is"),
      $._expression,
    )),

    percent_literal: $ => seq(
      $.number,
      choice("%", "percent", "percents", "pct", "pct."),
    ),

    function_call: $ => prec(17, choice(
      seq($.function_name, "(", $._expression, ")"),
      seq($.function_name, $._expression),
    )),

    function_name: $ => choice(
      "sqrt", "square root",
      "cbrt", "cubic root", "cube root", "cubed root",
      "abs",
      "round", "ceil", "floor",
      "log", "ln",
      "fact",
      "sin", "cos", "tan",
      "asin", "arcsin", "acos", "arccos", "atan", "arctan",
      "sinh", "cosh", "tanh",
    ),

    unary_expression: $ => prec(15, seq("-", $._expression)),

    parenthesized: $ => seq("(", $._expression, ")"),

    number_with_unit: $ => prec(20, seq($.number, $.unit_identifier)),

    currency_value: $ => prec(20, choice(
      seq($.currency_symbol, $.number),
      seq($.number, $.currency_symbol),
      seq($.number, $.currency_code),
    )),

    scaled_number: $ => prec(20, seq($.number, $.scale)),

    scale: $ => choice(
      "k", "thousand", "Thousand", "thousands",
      "million", "Million", "millions",
      "billion", "Billion", "milliard",
      "th", "th.",
    ),

    aggregation: $ => choice("sum", "total", "average", "avg", "prev"),

    number: $ => token(choice(
      // Hex
      /0[xX][0-9a-fA-F]+/,
      // Binary
      /0b[01]+/,
      // Octal
      /0[oO][0-7]+/,
      // Decimal with comma separators
      /\d{1,3}(,\d{3})+(\.\d+)?/,
      // Scientific notation
      /\d+\.?\d*[eE][+-]?\d+/,
      /\.\d+[eE][+-]?\d+/,
      // Regular decimal
      /\d+\.\d+/,
      /\d+/,
      // Leading dot
      /\.\d+/,
    )),

    variable: $ => $.identifier,

    identifier: $ => /[a-zA-Z_]\w*/,

    unit_identifier: $ => /[a-zA-Z][\w.]*(\s+[a-zA-Z][\w.]*)*/,

    currency_symbol: $ => choice(
      "$", "\u20AC", "\u00A3", "\u00A5", "\u20BD", "\u20AA",
      "\u20B9", "\u20A9", "\u20B4", "\u20BF", "\u20BA", "\u0E3F",
    ),

    currency_code: $ => choice(
      "USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD", "NZD", "CNY",
      "INR", "KRW", "RUB", "BRL", "MXN", "ZAR", "SEK", "NOK", "DKK",
      "PLN", "CZK", "HUF", "TRY", "SGD", "HKD", "THB", "AED", "SAR",
      "ILS", "UAH", "BTC", "ETH",
    ),
  },
});
