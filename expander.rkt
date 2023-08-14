#lang br/quicklang

(provide (matching-identifiers-out #rx"^b-" (all-defined-out)))

(provide (rename-out [b-module-begin #%module-begin]))

(define-macro (b-module-begin (b-program EXPRESSION))
  #'(#%module-begin
     (let ((res EXPRESSION))
       (print res))))




(define-macro (b-fun-arguments ARGUMENTS *...)
  #'(ARGUMENTS *...))

(define-macro (b-fun ARGS BODY)
  #'(lambda ARGS BODY))

