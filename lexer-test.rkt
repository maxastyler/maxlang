#lang br
(require "lexer.rkt" brag/support rackunit)

(define (lex str)
  (apply-port-proc maxlang-lexer str))

(check-equal? (lex "") empty)


