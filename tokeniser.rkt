#lang br
(require "lexer.rkt" brag/support)

(define (make-tokeniser ip [path #f])
  (port-count-lines! ip)
  (lexer-file-path path)
  (define (next-token) (maxlang-lexer ip))
  next-token)

(provide make-tokeniser)
