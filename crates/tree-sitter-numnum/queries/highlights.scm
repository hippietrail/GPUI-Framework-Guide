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
"=" @operator

;; Conversion keywords
(conversion ["in" "to" "as" "into"] @keyword)

;; Percent keywords
(percent_of ["of" "from"] @keyword)
(percent_on "on" @keyword)
(percent_off "off" @keyword)
(reverse_percent ["of what is" "on what is" "off what is"] @keyword)
(as_a_percent ["as a % of" "as a percent of"] @keyword)
(as_a_percent ["as a % on" "as a percent on"] @keyword)
(as_a_percent ["as a % off" "as a percent off"] @keyword)
(as_a_percent ["as a % increase of" "as a percent increase of"] @keyword)
(as_a_percent ["as a % decrease of" "as a percent decrease of"] @keyword)

;; Units
(unit_identifier) @type
(conversion target: (repr_keyword) @keyword)

;; Currency
(currency_symbol) @constant
(currency_code) @constant

;; Scale suffixes
(scale) @number

;; Aggregation
(aggregation) @function.builtin

;; Parentheses
["(" ")"] @punctuation.bracket
