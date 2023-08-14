#lang br
(require "lexer.rkt" brag/support rackunit)

(define (lex str)
  (apply-port-proc maxlang-lexer str))

(check-equal? (lex "")
              empty)


(check-equal? (lex " \n")
              (list (srcloc-token (token " \n" #:skip? #t)
                                  (srcloc 'string 1 0 1 2))))

(check-equal? (lex "(")
              (list (srcloc-token (token "(" "(")
                                  (srcloc 'string 1 0 1 1))))
