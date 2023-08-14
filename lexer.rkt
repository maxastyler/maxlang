#lang br
(require brag/support)

(define-lex-abbrev digits (:+ (char-set "0123456789")))

(define-lex-abbrev reserved-terms (:or "set" "=>" "cond" "fn" "(" ")" "{" "}" "`" ";" "," "!" "\""))

(define maxlang-lexer
  (lexer-srcloc
   [whitespace (token lexeme #:skip? #t)]
   [digits (token 'INTEGER (string->number lexeme))]
   [(:or (:seq (:? digits) "." digits)
         (:seq digits "."))
    (token 'DECIMAL (string->number lexeme))]
   [(:or (from/to "\"" "\"") (from/to "'" "'"))
    (token ('STRING
            (substring lexeme 1 (sub1 (string-length lexeme)))))]))

(provide maxlang-lexer)

