#lang brag
b-program : b-expr
b-fun-arguments : "("[SYMBOL ("," SYMBOL)*]")"
b-block : b-expr (";" b-expr)* [";"]
b-brace-block : "{" [b-block] "}"
b-paren-block : "(" [b-block] ")"
b-number : INTEGER | DECIMAL
b-lit : b-number | STRING 
b-fun : "fn" [SYMBOL] [b-fun-arguments] (b-brace-block | b-expr)
expr-l1 : b-lit | b-fun | b-brace-block | b-paren-block | SYMBOL
b-infix-call-inner : "`" expr-l1+
b-infix-call : expr-l1 b-infix-call-inner+
expr-l2 : b-infix-call | expr-l1
b-call : expr-l2 (expr-l2)+
expr-l3 : b-call | expr-l2
b-no-arg-call : b-expr "!"
b-expr : b-assignment | b-no-arg-call | expr-l3
b-assignment : "set" SYMBOL b-expr
