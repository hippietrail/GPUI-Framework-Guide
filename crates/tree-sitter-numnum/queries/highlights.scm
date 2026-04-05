;; numnum syntax highlighting queries

;; Comments and headers
(comment) @comment
(header) @markup.heading

;; Labels
(label (identifier) @label)

;; Numbers
(number) @number

;; Percent symbol
(percent_literal "%") @operator
(percent_literal "percent") @operator
(percent_literal "percents") @operator
(percent_literal "pct") @operator
(percent_literal "pct.") @operator

;; Functions
(function_name) @function.builtin

;; Variables
(variable (identifier) @variable)
(assignment name: (identifier) @variable.definition)
(compound_assignment name: (identifier) @variable.definition)

;; Operators
(binary_expression operator: _ @operator)
(compound_assignment operator: _ @operator)
(inline_percent operator: _ @operator)

;; Keywords (conversion, percent)
["in" "to" "as" "into"] @keyword
["of" "from" "on" "off"] @keyword
["of what is" "on what is" "off what is"] @keyword
["as a % of" "as a percent of"] @keyword
["as a % on" "as a percent on"] @keyword
["as a % off" "as a percent off"] @keyword
["as a % increase of" "as a percent increase of"] @keyword
["as a % decrease of" "as a percent decrease of"] @keyword

;; Assignment keywords
["=" "equal" "is"] @operator

;; Units
(unit_identifier) @type
(number_with_unit (unit_identifier) @type)
(conversion target: (unit_identifier) @type)

;; Repr keywords
(repr_keyword) @keyword

;; Currency
(currency_symbol) @constant
(currency_code) @constant

;; Scale suffixes
(scale) @number

;; Aggregation
(aggregation) @function.builtin

;; Parentheses
["(" ")"] @punctuation.bracket
