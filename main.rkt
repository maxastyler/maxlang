#lang br/quicklang
(require "parser.rkt" "tokeniser.rkt")

(define (read-syntax path port)
  (define parse-tree (parse path (make-tokeniser port path)))
  (strip-bindings
   #`(module maxlang-mod maxlang/expander
       #,parse-tree)))

(module+ reader
  (provide read-syntax))
