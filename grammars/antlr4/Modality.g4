grammar Modality;

expression: f = formula EOF;

formula:
	TRUE												                                                                   # trueAtom
	| FALSE												                                                                 # falseAtom
	| left = formula OR right = formula              					                                     # orFormula
	| left = formula AND right = formula              				                                     # andFormula
	| MINUS inner = formula																																				 # negatedFormula
	| NOT inner = formula								                                                           # notFormula
	| (unsignedProp (signedProp)* | (signedProp)+)						                                     # propsSet
	| STATE_SET_VARIABLE                                                                           # stateSetVariable
  | LBOX RBOX outer = formula					                                                           # emptyBoxFormula
	| LDIA RDIA outer = formula					                                                           # emptyDiamondFormula
	| LBOX inner = formula RBOX outer = formula					                                           # boxFormula
	| LDIA inner = formula RDIA outer = formula					                                           # diamondFormula
	| LFP LPAREN stateSetVariable = formula COMMA inner = formula RPAREN	                         # lfpFormula
	| GFP LPAREN stateSetVariable = formula COMMA inner = formula RPAREN	                         # gfpFormula
	| MUST LPAREN formula RPAREN                                                                   # mustMacro
	| CAN LPAREN formula RPAREN                                                                    # canMacro
	| ALWAYS LPAREN inner_formula = formula RPAREN (UNTIL LPAREN until_formula = formula RPAREN)?  # alwaysMacro
	| EVENTUALLY LPAREN inner_formula = formula RPAREN (UNTIL LPAREN until_formula = formula RPAREN)? # eventuallyMacro
	| UNTIL LPAREN pre_formula = formula COMMA post_formula = formula RPAREN                       # untilMacro
	| WHEN when_formula = formula ALSO also_formula = formula                                      # whenAlsoFormula
	| WHEN when_formula = formula NEXT next_formula = formula                                      # whenNextFormula
	| LPAREN inner = formula RPAREN						                                                     # parenFormula
	;

functionProp: name = NAME LPAREN (arg (',' arg)*)? RPAREN;

unsignedProp: theProp = prop;
signedProp: (theSign = sign) WS* theProp = prop;
sign: PLUS | MINUS | QMARK # sign;
prop: TRUE | FALSE | NAME | functionProp;

arg:
	TRUE # trueArg
	| FALSE # falseArg
	| STRING # stringArg
	| NUMBER # numberArg
	| VARIABLE # variableArg
	;

MUST: 'must';
CAN: 'can';

ALWAYS: 'always';
EVENTUALLY: 'eventually';
UNTIL: 'until';

TRUE: 'true';
FALSE: 'false';
AND: 'and';
OR: 'or';
NOT: 'not';
WHEN: 'when';
ALSO: 'also';
NEXT: 'next';

LBOX: '[';
RBOX: ']';
LDIA: '<';
RDIA: '>';
LPAREN: '(';
RPAREN: ')';

STAR: '*';

COMMA: ',';

PLUS: '+';
MINUS: '-';
QMARK: '?';

LFP: 'lfp';
GFP: 'gfp';

NAME: [A-Za-z][A-Za-z0-9_]*;

STRING: '"' (ESC | SAFECODEPOINT)* '"';
fragment ESC: '\\' (["\\/bfnrt] | UNICODE);
fragment UNICODE: 'u' HEX HEX HEX HEX;
fragment HEX: [0-9a-fA-F];
fragment SAFECODEPOINT: ~ ["\\\u0000-\u001F];

NUMBER: '-'? INT ('.' [0-9]+)? EXP?;
fragment INT: '0' | [1-9] [0-9]*;
fragment EXP: [Ee] [+\-]? INT;

STATE_SET_VARIABLE: [@][A-Za-z0-9_]*;
VARIABLE: [A-Za-z0-9_/][A-Za-z0-9_/.]*;

// whitespace
WS: [ \n\t\r]+ -> skip;
LINE_COMMENT: '//' ~[\r\n]* -> skip;
