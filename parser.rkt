#lang brag
b-program : b-expr



b-assignment-chain : (b-expr /";")* b-assignment
@b-block : (b-assignment-chain /";")* (b-assignment | ((b-expr /";")* [b-expr]))
b-brace-block : /"{" [b-block] /"}"
@b-number : INTEGER | DECIMAL
@b-lit : STRING | b-number
b-fun-arguments : /"("[SYMBOL (/"," SYMBOL)*]/")"
b-fun : b-fun-arguments (b-brace-block | b-expr)
@expr-l1 : b-lit | b-fun | b-brace-block | SYMBOL
b-infix-call-inner : /"`" expr-l1+
b-infix-call : expr-l1 b-infix-call-inner+
@expr-l2 : b-infix-call | expr-l1
b-call : expr-l2 (expr-l2)+
@expr-l3 : b-call | expr-l2
b-no-arg-call : b-expr /"!"
@b-expr : b-no-arg-call | expr-l3
b-assignment : /"let" SYMBOL b-expr
