#lang br
(require brag/support)

(define-lex-abbrev digit (char-set "0123456789"))
(define-lex-abbrev reserved-char (char-set "(){}`;,!\""))

(define-lex-abbrev digits (:+ digit))

(define-lex-abbrev reserved-term (:or "let" "=>" "cond" reserved-char))

(define maxlang-lexer
  (lexer-srcloc
   [(:+ whitespace) (token lexeme #:skip? #t)]
   [reserved-term (token lexeme lexeme)]
   [(:seq alphabetic (:* (:or alphabetic numeric))) (token 'SYMBOL (string->symbol lexeme))]
   [digits (token 'INTEGER (string->number lexeme))]
   [(:or (:seq (:? digits) "." digits)
         (:seq digits "."))
    (token 'DECIMAL (string->number lexeme))]
   [(:or (from/to "\"" "\"") (from/to "'" "'"))
    (token ('STRING
            (substring lexeme 1 (sub1 (string-length lexeme)))))]))

(provide maxlang-lexer)

