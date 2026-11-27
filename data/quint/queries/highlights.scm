; Highlight queries for tree-sitter-quint.
; Captures use the standard tree-sitter / Neovim conventions; Zed maps
; the same names to its theme scopes.

(keyword) @keyword
(storage_modifier) @keyword.storage
(storage_type) @type.builtin
(constant) @constant.builtin

(operator) @operator

(string) @string
(escape_sequence) @string.escape

(number) @number

(line_comment) @comment
(block_comment) @comment
(hashbang) @comment

; Quint convention: capitalized identifiers are types or sum constructors.
((identifier) @type
 (#match? @type "^[A-Z]"))
