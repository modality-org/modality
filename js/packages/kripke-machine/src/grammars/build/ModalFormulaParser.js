// Generated from ../../../../grammars/ModalFormula.g4 by ANTLR 4.13.0
// jshint ignore: start
import antlr4 from 'antlr4';
import ModalFormulaListener from './ModalFormulaListener.js';
import ModalFormulaVisitor from './ModalFormulaVisitor.js';

const serializedATN = [4,1,20,98,2,0,7,0,2,1,7,1,2,2,7,2,2,3,7,3,2,4,7,4,
2,5,7,5,1,0,1,0,1,0,1,1,1,1,1,1,1,1,1,1,1,1,5,1,22,8,1,10,1,12,1,25,9,1,
1,1,4,1,28,8,1,11,1,12,1,29,3,1,32,8,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,1,70,8,1,1,1,1,1,1,1,1,1,1,1,1,1,
5,1,78,8,1,10,1,12,1,81,9,1,1,2,1,2,1,3,1,3,5,3,87,8,3,10,3,12,3,90,9,3,
1,3,1,3,1,4,1,4,1,5,1,5,1,5,0,1,2,6,0,2,4,6,8,10,0,2,1,0,7,8,2,0,1,2,17,
17,108,0,12,1,0,0,0,2,69,1,0,0,0,4,82,1,0,0,0,6,84,1,0,0,0,8,93,1,0,0,0,
10,95,1,0,0,0,12,13,3,2,1,0,13,14,5,0,0,1,14,1,1,0,0,0,15,16,6,1,-1,0,16,
70,5,1,0,0,17,70,5,2,0,0,18,70,5,18,0,0,19,23,3,4,2,0,20,22,3,6,3,0,21,20,
1,0,0,0,22,25,1,0,0,0,23,21,1,0,0,0,23,24,1,0,0,0,24,32,1,0,0,0,25,23,1,
0,0,0,26,28,3,6,3,0,27,26,1,0,0,0,28,29,1,0,0,0,29,27,1,0,0,0,29,30,1,0,
0,0,30,32,1,0,0,0,31,19,1,0,0,0,31,27,1,0,0,0,32,70,1,0,0,0,33,34,5,9,0,
0,34,35,5,10,0,0,35,70,3,2,1,10,36,37,5,11,0,0,37,38,5,12,0,0,38,70,3,2,
1,9,39,40,5,9,0,0,40,41,3,2,1,0,41,42,5,10,0,0,42,43,3,2,1,8,43,70,1,0,0,
0,44,45,5,11,0,0,45,46,3,2,1,0,46,47,5,12,0,0,47,48,3,2,1,7,48,70,1,0,0,
0,49,50,5,15,0,0,50,51,5,13,0,0,51,52,3,2,1,0,52,53,5,6,0,0,53,54,3,2,1,
0,54,55,5,14,0,0,55,70,1,0,0,0,56,57,5,16,0,0,57,58,5,13,0,0,58,59,3,2,1,
0,59,60,5,6,0,0,60,61,3,2,1,0,61,62,5,14,0,0,62,70,1,0,0,0,63,64,5,5,0,0,
64,70,3,2,1,2,65,66,5,13,0,0,66,67,3,2,1,0,67,68,5,14,0,0,68,70,1,0,0,0,
69,15,1,0,0,0,69,17,1,0,0,0,69,18,1,0,0,0,69,31,1,0,0,0,69,33,1,0,0,0,69,
36,1,0,0,0,69,39,1,0,0,0,69,44,1,0,0,0,69,49,1,0,0,0,69,56,1,0,0,0,69,63,
1,0,0,0,69,65,1,0,0,0,70,79,1,0,0,0,71,72,10,4,0,0,72,73,5,4,0,0,73,78,3,
2,1,5,74,75,10,3,0,0,75,76,5,3,0,0,76,78,3,2,1,4,77,71,1,0,0,0,77,74,1,0,
0,0,78,81,1,0,0,0,79,77,1,0,0,0,79,80,1,0,0,0,80,3,1,0,0,0,81,79,1,0,0,0,
82,83,3,10,5,0,83,5,1,0,0,0,84,88,3,8,4,0,85,87,5,19,0,0,86,85,1,0,0,0,87,
90,1,0,0,0,88,86,1,0,0,0,88,89,1,0,0,0,89,91,1,0,0,0,90,88,1,0,0,0,91,92,
3,10,5,0,92,7,1,0,0,0,93,94,7,0,0,0,94,9,1,0,0,0,95,96,7,1,0,0,96,11,1,0,
0,0,7,23,29,31,69,77,79,88];


const atn = new antlr4.atn.ATNDeserializer().deserialize(serializedATN);

const decisionsToDFA = atn.decisionToState.map( (ds, index) => new antlr4.dfa.DFA(ds, index) );

const sharedContextCache = new antlr4.atn.PredictionContextCache();

export default class ModalFormulaParser extends antlr4.Parser {

    static grammarFileName = "ModalFormula.g4";
    static literalNames = [ null, "'true'", "'false'", "'and'", "'or'", 
                            "'not'", "','", "'+'", "'-'", "'['", "']'", 
                            "'<'", "'>'", "'('", "')'", "'lfp'", "'gfp'" ];
    static symbolicNames = [ null, "TRUE", "FALSE", "AND", "OR", "NOT", 
                             "COMMA", "PLUS", "MINUS", "LBOX", "RBOX", "LDIA", 
                             "RDIA", "LPAREN", "RPAREN", "LFP", "GFP", "NAME", 
                             "BOUND_VAR", "WS", "LINE_COMMENT" ];
    static ruleNames = [ "formula", "subformula", "unsignedProp", "signedProp", 
                         "sign", "prop" ];

    constructor(input) {
        super(input);
        this._interp = new antlr4.atn.ParserATNSimulator(this, atn, decisionsToDFA, sharedContextCache);
        this.ruleNames = ModalFormulaParser.ruleNames;
        this.literalNames = ModalFormulaParser.literalNames;
        this.symbolicNames = ModalFormulaParser.symbolicNames;
    }

    sempred(localctx, ruleIndex, predIndex) {
    	switch(ruleIndex) {
    	case 1:
    	    		return this.subformula_sempred(localctx, predIndex);
        default:
            throw "No predicate with index:" + ruleIndex;
       }
    }

    subformula_sempred(localctx, predIndex) {
    	switch(predIndex) {
    		case 0:
    			return this.precpred(this._ctx, 4);
    		case 1:
    			return this.precpred(this._ctx, 3);
    		default:
    			throw "No predicate with index:" + predIndex;
    	}
    };




	formula() {
	    let localctx = new FormulaContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 0, ModalFormulaParser.RULE_formula);
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 12;
	        localctx.f = this.subformula(0);
	        this.state = 13;
	        this.match(ModalFormulaParser.EOF);
	    } catch (re) {
	    	if(re instanceof antlr4.error.RecognitionException) {
		        localctx.exception = re;
		        this._errHandler.reportError(this, re);
		        this._errHandler.recover(this, re);
		    } else {
		    	throw re;
		    }
	    } finally {
	        this.exitRule();
	    }
	    return localctx;
	}


	subformula(_p) {
		if(_p===undefined) {
		    _p = 0;
		}
	    const _parentctx = this._ctx;
	    const _parentState = this.state;
	    let localctx = new SubformulaContext(this, this._ctx, _parentState);
	    let _prevctx = localctx;
	    const _startState = 2;
	    this.enterRecursionRule(localctx, 2, ModalFormulaParser.RULE_subformula, _p);
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 69;
	        this._errHandler.sync(this);
	        var la_ = this._interp.adaptivePredict(this._input,3,this._ctx);
	        switch(la_) {
	        case 1:
	            localctx = new TrueAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;

	            this.state = 16;
	            this.match(ModalFormulaParser.TRUE);
	            break;

	        case 2:
	            localctx = new FalseAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 17;
	            this.match(ModalFormulaParser.FALSE);
	            break;

	        case 3:
	            localctx = new BoundVarContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 18;
	            this.match(ModalFormulaParser.BOUND_VAR);
	            break;

	        case 4:
	            localctx = new PropsAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 31;
	            this._errHandler.sync(this);
	            switch(this._input.LA(1)) {
	            case 1:
	            case 2:
	            case 17:
	                this.state = 19;
	                this.unsignedProp();
	                this.state = 23;
	                this._errHandler.sync(this);
	                var _alt = this._interp.adaptivePredict(this._input,0,this._ctx)
	                while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	                    if(_alt===1) {
	                        this.state = 20;
	                        this.signedProp(); 
	                    }
	                    this.state = 25;
	                    this._errHandler.sync(this);
	                    _alt = this._interp.adaptivePredict(this._input,0,this._ctx);
	                }

	                break;
	            case 7:
	            case 8:
	                this.state = 27; 
	                this._errHandler.sync(this);
	                var _alt = 1;
	                do {
	                	switch (_alt) {
	                	case 1:
	                		this.state = 26;
	                		this.signedProp();
	                		break;
	                	default:
	                		throw new antlr4.error.NoViableAltException(this);
	                	}
	                	this.state = 29; 
	                	this._errHandler.sync(this);
	                	_alt = this._interp.adaptivePredict(this._input,1, this._ctx);
	                } while ( _alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER );
	                break;
	            default:
	                throw new antlr4.error.NoViableAltException(this);
	            }
	            break;

	        case 5:
	            localctx = new EmptyBoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 33;
	            this.match(ModalFormulaParser.LBOX);
	            this.state = 34;
	            this.match(ModalFormulaParser.RBOX);
	            this.state = 35;
	            localctx.outer = this.subformula(10);
	            break;

	        case 6:
	            localctx = new EmptyDiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 36;
	            this.match(ModalFormulaParser.LDIA);
	            this.state = 37;
	            this.match(ModalFormulaParser.RDIA);
	            this.state = 38;
	            localctx.outer = this.subformula(9);
	            break;

	        case 7:
	            localctx = new BoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 39;
	            this.match(ModalFormulaParser.LBOX);
	            this.state = 40;
	            localctx.inner = this.subformula(0);
	            this.state = 41;
	            this.match(ModalFormulaParser.RBOX);
	            this.state = 42;
	            localctx.outer = this.subformula(8);
	            break;

	        case 8:
	            localctx = new DiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 44;
	            this.match(ModalFormulaParser.LDIA);
	            this.state = 45;
	            localctx.inner = this.subformula(0);
	            this.state = 46;
	            this.match(ModalFormulaParser.RDIA);
	            this.state = 47;
	            localctx.outer = this.subformula(7);
	            break;

	        case 9:
	            localctx = new LfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 49;
	            this.match(ModalFormulaParser.LFP);
	            this.state = 50;
	            this.match(ModalFormulaParser.LPAREN);
	            this.state = 51;
	            localctx.boundVar = this.subformula(0);
	            this.state = 52;
	            this.match(ModalFormulaParser.COMMA);
	            this.state = 53;
	            localctx.inner = this.subformula(0);
	            this.state = 54;
	            this.match(ModalFormulaParser.RPAREN);
	            break;

	        case 10:
	            localctx = new GfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 56;
	            this.match(ModalFormulaParser.GFP);
	            this.state = 57;
	            this.match(ModalFormulaParser.LPAREN);
	            this.state = 58;
	            localctx.boundVar = this.subformula(0);
	            this.state = 59;
	            this.match(ModalFormulaParser.COMMA);
	            this.state = 60;
	            localctx.inner = this.subformula(0);
	            this.state = 61;
	            this.match(ModalFormulaParser.RPAREN);
	            break;

	        case 11:
	            localctx = new NotFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 63;
	            this.match(ModalFormulaParser.NOT);
	            this.state = 64;
	            localctx.inner = this.subformula(2);
	            break;

	        case 12:
	            localctx = new ParenFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 65;
	            this.match(ModalFormulaParser.LPAREN);
	            this.state = 66;
	            localctx.inner = this.subformula(0);
	            this.state = 67;
	            this.match(ModalFormulaParser.RPAREN);
	            break;

	        }
	        this._ctx.stop = this._input.LT(-1);
	        this.state = 79;
	        this._errHandler.sync(this);
	        var _alt = this._interp.adaptivePredict(this._input,5,this._ctx)
	        while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	            if(_alt===1) {
	                if(this._parseListeners!==null) {
	                    this.triggerExitRuleEvent();
	                }
	                _prevctx = localctx;
	                this.state = 77;
	                this._errHandler.sync(this);
	                var la_ = this._interp.adaptivePredict(this._input,4,this._ctx);
	                switch(la_) {
	                case 1:
	                    localctx = new OrFormulaContext(this, new SubformulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalFormulaParser.RULE_subformula);
	                    this.state = 71;
	                    if (!( this.precpred(this._ctx, 4))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 4)");
	                    }
	                    this.state = 72;
	                    this.match(ModalFormulaParser.OR);
	                    this.state = 73;
	                    localctx.right = this.subformula(5);
	                    break;

	                case 2:
	                    localctx = new AndFormulaContext(this, new SubformulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalFormulaParser.RULE_subformula);
	                    this.state = 74;
	                    if (!( this.precpred(this._ctx, 3))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 3)");
	                    }
	                    this.state = 75;
	                    this.match(ModalFormulaParser.AND);
	                    this.state = 76;
	                    localctx.right = this.subformula(4);
	                    break;

	                } 
	            }
	            this.state = 81;
	            this._errHandler.sync(this);
	            _alt = this._interp.adaptivePredict(this._input,5,this._ctx);
	        }

	    } catch( error) {
	        if(error instanceof antlr4.error.RecognitionException) {
		        localctx.exception = error;
		        this._errHandler.reportError(this, error);
		        this._errHandler.recover(this, error);
		    } else {
		    	throw error;
		    }
	    } finally {
	        this.unrollRecursionContexts(_parentctx)
	    }
	    return localctx;
	}



	unsignedProp() {
	    let localctx = new UnsignedPropContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 4, ModalFormulaParser.RULE_unsignedProp);
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 82;
	        localctx.theProp = this.prop();
	    } catch (re) {
	    	if(re instanceof antlr4.error.RecognitionException) {
		        localctx.exception = re;
		        this._errHandler.reportError(this, re);
		        this._errHandler.recover(this, re);
		    } else {
		    	throw re;
		    }
	    } finally {
	        this.exitRule();
	    }
	    return localctx;
	}



	signedProp() {
	    let localctx = new SignedPropContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 6, ModalFormulaParser.RULE_signedProp);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 84;
	        localctx.theSign = this.sign();
	        this.state = 88;
	        this._errHandler.sync(this);
	        _la = this._input.LA(1);
	        while(_la===19) {
	            this.state = 85;
	            this.match(ModalFormulaParser.WS);
	            this.state = 90;
	            this._errHandler.sync(this);
	            _la = this._input.LA(1);
	        }
	        this.state = 91;
	        localctx.theProp = this.prop();
	    } catch (re) {
	    	if(re instanceof antlr4.error.RecognitionException) {
		        localctx.exception = re;
		        this._errHandler.reportError(this, re);
		        this._errHandler.recover(this, re);
		    } else {
		    	throw re;
		    }
	    } finally {
	        this.exitRule();
	    }
	    return localctx;
	}



	sign() {
	    let localctx = new SignContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 8, ModalFormulaParser.RULE_sign);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 93;
	        _la = this._input.LA(1);
	        if(!(_la===7 || _la===8)) {
	        this._errHandler.recoverInline(this);
	        }
	        else {
	        	this._errHandler.reportMatch(this);
	            this.consume();
	        }
	    } catch (re) {
	    	if(re instanceof antlr4.error.RecognitionException) {
		        localctx.exception = re;
		        this._errHandler.reportError(this, re);
		        this._errHandler.recover(this, re);
		    } else {
		    	throw re;
		    }
	    } finally {
	        this.exitRule();
	    }
	    return localctx;
	}



	prop() {
	    let localctx = new PropContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 10, ModalFormulaParser.RULE_prop);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 95;
	        _la = this._input.LA(1);
	        if(!((((_la) & ~0x1f) === 0 && ((1 << _la) & 131078) !== 0))) {
	        this._errHandler.recoverInline(this);
	        }
	        else {
	        	this._errHandler.reportMatch(this);
	            this.consume();
	        }
	    } catch (re) {
	    	if(re instanceof antlr4.error.RecognitionException) {
		        localctx.exception = re;
		        this._errHandler.reportError(this, re);
		        this._errHandler.recover(this, re);
		    } else {
		    	throw re;
		    }
	    } finally {
	        this.exitRule();
	    }
	    return localctx;
	}


}

ModalFormulaParser.EOF = antlr4.Token.EOF;
ModalFormulaParser.TRUE = 1;
ModalFormulaParser.FALSE = 2;
ModalFormulaParser.AND = 3;
ModalFormulaParser.OR = 4;
ModalFormulaParser.NOT = 5;
ModalFormulaParser.COMMA = 6;
ModalFormulaParser.PLUS = 7;
ModalFormulaParser.MINUS = 8;
ModalFormulaParser.LBOX = 9;
ModalFormulaParser.RBOX = 10;
ModalFormulaParser.LDIA = 11;
ModalFormulaParser.RDIA = 12;
ModalFormulaParser.LPAREN = 13;
ModalFormulaParser.RPAREN = 14;
ModalFormulaParser.LFP = 15;
ModalFormulaParser.GFP = 16;
ModalFormulaParser.NAME = 17;
ModalFormulaParser.BOUND_VAR = 18;
ModalFormulaParser.WS = 19;
ModalFormulaParser.LINE_COMMENT = 20;

ModalFormulaParser.RULE_formula = 0;
ModalFormulaParser.RULE_subformula = 1;
ModalFormulaParser.RULE_unsignedProp = 2;
ModalFormulaParser.RULE_signedProp = 3;
ModalFormulaParser.RULE_sign = 4;
ModalFormulaParser.RULE_prop = 5;

class FormulaContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalFormulaParser.RULE_formula;
        this.f = null;
    }

	EOF() {
	    return this.getToken(ModalFormulaParser.EOF, 0);
	};

	subformula() {
	    return this.getTypedRuleContext(SubformulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



class SubformulaContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalFormulaParser.RULE_subformula;
    }


	 
		copyFrom(ctx) {
			super.copyFrom(ctx);
		}

}


class EmptyBoxFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LBOX() {
	    return this.getToken(ModalFormulaParser.LBOX, 0);
	};

	RBOX() {
	    return this.getToken(ModalFormulaParser.RBOX, 0);
	};

	subformula() {
	    return this.getTypedRuleContext(SubformulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterEmptyBoxFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitEmptyBoxFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitEmptyBoxFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.EmptyBoxFormulaContext = EmptyBoxFormulaContext;

class BoundVarContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	BOUND_VAR() {
	    return this.getToken(ModalFormulaParser.BOUND_VAR, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterBoundVar(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitBoundVar(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitBoundVar(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.BoundVarContext = BoundVarContext;

class EmptyDiamondFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LDIA() {
	    return this.getToken(ModalFormulaParser.LDIA, 0);
	};

	RDIA() {
	    return this.getToken(ModalFormulaParser.RDIA, 0);
	};

	subformula() {
	    return this.getTypedRuleContext(SubformulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterEmptyDiamondFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitEmptyDiamondFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitEmptyDiamondFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.EmptyDiamondFormulaContext = EmptyDiamondFormulaContext;

class AndFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.left = null;;
        this.right = null;;
        super.copyFrom(ctx);
    }

	AND() {
	    return this.getToken(ModalFormulaParser.AND, 0);
	};

	subformula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(SubformulaContext);
	    } else {
	        return this.getTypedRuleContext(SubformulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterAndFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitAndFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitAndFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.AndFormulaContext = AndFormulaContext;

class NotFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        super.copyFrom(ctx);
    }

	NOT() {
	    return this.getToken(ModalFormulaParser.NOT, 0);
	};

	subformula() {
	    return this.getTypedRuleContext(SubformulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterNotFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitNotFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitNotFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.NotFormulaContext = NotFormulaContext;

class OrFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.left = null;;
        this.right = null;;
        super.copyFrom(ctx);
    }

	OR() {
	    return this.getToken(ModalFormulaParser.OR, 0);
	};

	subformula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(SubformulaContext);
	    } else {
	        return this.getTypedRuleContext(SubformulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterOrFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitOrFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitOrFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.OrFormulaContext = OrFormulaContext;

class LfpFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.boundVar = null;;
        this.inner = null;;
        super.copyFrom(ctx);
    }

	LFP() {
	    return this.getToken(ModalFormulaParser.LFP, 0);
	};

	LPAREN() {
	    return this.getToken(ModalFormulaParser.LPAREN, 0);
	};

	COMMA() {
	    return this.getToken(ModalFormulaParser.COMMA, 0);
	};

	RPAREN() {
	    return this.getToken(ModalFormulaParser.RPAREN, 0);
	};

	subformula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(SubformulaContext);
	    } else {
	        return this.getTypedRuleContext(SubformulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterLfpFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitLfpFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitLfpFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.LfpFormulaContext = LfpFormulaContext;

class DiamondFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LDIA() {
	    return this.getToken(ModalFormulaParser.LDIA, 0);
	};

	RDIA() {
	    return this.getToken(ModalFormulaParser.RDIA, 0);
	};

	subformula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(SubformulaContext);
	    } else {
	        return this.getTypedRuleContext(SubformulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterDiamondFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitDiamondFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitDiamondFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.DiamondFormulaContext = DiamondFormulaContext;

class ParenFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        super.copyFrom(ctx);
    }

	LPAREN() {
	    return this.getToken(ModalFormulaParser.LPAREN, 0);
	};

	RPAREN() {
	    return this.getToken(ModalFormulaParser.RPAREN, 0);
	};

	subformula() {
	    return this.getTypedRuleContext(SubformulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterParenFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitParenFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitParenFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.ParenFormulaContext = ParenFormulaContext;

class BoxFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LBOX() {
	    return this.getToken(ModalFormulaParser.LBOX, 0);
	};

	RBOX() {
	    return this.getToken(ModalFormulaParser.RBOX, 0);
	};

	subformula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(SubformulaContext);
	    } else {
	        return this.getTypedRuleContext(SubformulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterBoxFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitBoxFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitBoxFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.BoxFormulaContext = BoxFormulaContext;

class FalseAtomContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	FALSE() {
	    return this.getToken(ModalFormulaParser.FALSE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterFalseAtom(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitFalseAtom(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitFalseAtom(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.FalseAtomContext = FalseAtomContext;

class GfpFormulaContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.boundVar = null;;
        this.inner = null;;
        super.copyFrom(ctx);
    }

	GFP() {
	    return this.getToken(ModalFormulaParser.GFP, 0);
	};

	LPAREN() {
	    return this.getToken(ModalFormulaParser.LPAREN, 0);
	};

	COMMA() {
	    return this.getToken(ModalFormulaParser.COMMA, 0);
	};

	RPAREN() {
	    return this.getToken(ModalFormulaParser.RPAREN, 0);
	};

	subformula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(SubformulaContext);
	    } else {
	        return this.getTypedRuleContext(SubformulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterGfpFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitGfpFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitGfpFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.GfpFormulaContext = GfpFormulaContext;

class TrueAtomContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	TRUE() {
	    return this.getToken(ModalFormulaParser.TRUE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterTrueAtom(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitTrueAtom(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitTrueAtom(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.TrueAtomContext = TrueAtomContext;

class PropsAtomContext extends SubformulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	unsignedProp() {
	    return this.getTypedRuleContext(UnsignedPropContext,0);
	};

	signedProp = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(SignedPropContext);
	    } else {
	        return this.getTypedRuleContext(SignedPropContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterPropsAtom(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitPropsAtom(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitPropsAtom(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalFormulaParser.PropsAtomContext = PropsAtomContext;

class UnsignedPropContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalFormulaParser.RULE_unsignedProp;
        this.theProp = null;
    }

	prop() {
	    return this.getTypedRuleContext(PropContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterUnsignedProp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitUnsignedProp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitUnsignedProp(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



class SignedPropContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalFormulaParser.RULE_signedProp;
        this.theSign = null;
        this.theProp = null;
    }

	prop() {
	    return this.getTypedRuleContext(PropContext,0);
	};

	sign() {
	    return this.getTypedRuleContext(SignContext,0);
	};

	WS = function(i) {
		if(i===undefined) {
			i = null;
		}
	    if(i===null) {
	        return this.getTokens(ModalFormulaParser.WS);
	    } else {
	        return this.getToken(ModalFormulaParser.WS, i);
	    }
	};


	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterSignedProp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitSignedProp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitSignedProp(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



class SignContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalFormulaParser.RULE_sign;
    }

	PLUS() {
	    return this.getToken(ModalFormulaParser.PLUS, 0);
	};

	MINUS() {
	    return this.getToken(ModalFormulaParser.MINUS, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterSign(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitSign(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitSign(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



class PropContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalFormulaParser.RULE_prop;
    }

	TRUE() {
	    return this.getToken(ModalFormulaParser.TRUE, 0);
	};

	FALSE() {
	    return this.getToken(ModalFormulaParser.FALSE, 0);
	};

	NAME() {
	    return this.getToken(ModalFormulaParser.NAME, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.enterProp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalFormulaListener ) {
	        listener.exitProp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalFormulaVisitor ) {
	        return visitor.visitProp(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}




ModalFormulaParser.FormulaContext = FormulaContext; 
ModalFormulaParser.SubformulaContext = SubformulaContext; 
ModalFormulaParser.UnsignedPropContext = UnsignedPropContext; 
ModalFormulaParser.SignedPropContext = SignedPropContext; 
ModalFormulaParser.SignContext = SignContext; 
ModalFormulaParser.PropContext = PropContext; 
