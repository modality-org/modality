// Generated from ../../../../grammars/Modality.g4 by ANTLR 4.13.0
// jshint ignore: start
import antlr4 from 'antlr4';
import ModalityListener from './ModalityListener.js';
import ModalityVisitor from './ModalityVisitor.js';

const serializedATN = [4,1,33,170,2,0,7,0,2,1,7,1,2,2,7,2,2,3,7,3,2,4,7,
4,2,5,7,5,2,6,7,6,2,7,7,7,1,0,1,0,1,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,5,1,29,8,1,10,1,12,1,32,9,1,1,1,4,1,35,8,1,11,1,12,1,36,3,1,39,8,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,1,91,8,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,3,1,101,8,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,3,1,117,8,1,1,1,1,1,1,1,1,1,1,1,1,1,5,1,125,8,1,10,1,12,
1,128,9,1,1,2,1,2,1,2,1,2,1,2,5,2,135,8,2,10,2,12,2,138,9,2,3,2,140,8,2,
1,2,1,2,1,3,1,3,1,4,1,4,5,4,148,8,4,10,4,12,4,151,9,4,1,4,1,4,1,5,1,5,1,
6,1,6,1,6,1,6,3,6,161,8,6,1,7,1,7,1,7,1,7,1,7,3,7,168,8,7,1,7,0,1,2,8,0,
2,4,6,8,10,12,14,0,1,1,0,22,24,196,0,16,1,0,0,0,2,116,1,0,0,0,4,129,1,0,
0,0,6,143,1,0,0,0,8,145,1,0,0,0,10,154,1,0,0,0,12,160,1,0,0,0,14,167,1,0,
0,0,16,17,3,2,1,0,17,18,5,0,0,1,18,1,1,0,0,0,19,20,6,1,-1,0,20,117,5,6,0,
0,21,117,5,7,0,0,22,23,5,23,0,0,23,117,3,2,1,17,24,25,5,10,0,0,25,117,3,
2,1,16,26,30,3,6,3,0,27,29,3,8,4,0,28,27,1,0,0,0,29,32,1,0,0,0,30,28,1,0,
0,0,30,31,1,0,0,0,31,39,1,0,0,0,32,30,1,0,0,0,33,35,3,8,4,0,34,33,1,0,0,
0,35,36,1,0,0,0,36,34,1,0,0,0,36,37,1,0,0,0,37,39,1,0,0,0,38,26,1,0,0,0,
38,34,1,0,0,0,39,117,1,0,0,0,40,117,5,30,0,0,41,42,5,14,0,0,42,43,5,15,0,
0,43,117,3,2,1,13,44,45,5,16,0,0,45,46,5,17,0,0,46,117,3,2,1,12,47,48,5,
14,0,0,48,49,3,2,1,0,49,50,5,15,0,0,50,51,3,2,1,11,51,117,1,0,0,0,52,53,
5,16,0,0,53,54,3,2,1,0,54,55,5,17,0,0,55,56,3,2,1,10,56,117,1,0,0,0,57,58,
5,25,0,0,58,59,5,18,0,0,59,60,3,2,1,0,60,61,5,21,0,0,61,62,3,2,1,0,62,63,
5,19,0,0,63,117,1,0,0,0,64,65,5,26,0,0,65,66,5,18,0,0,66,67,3,2,1,0,67,68,
5,21,0,0,68,69,3,2,1,0,69,70,5,19,0,0,70,117,1,0,0,0,71,72,5,1,0,0,72,73,
5,18,0,0,73,74,3,2,1,0,74,75,5,19,0,0,75,117,1,0,0,0,76,77,5,2,0,0,77,78,
5,18,0,0,78,79,3,2,1,0,79,80,5,19,0,0,80,117,1,0,0,0,81,82,5,3,0,0,82,83,
5,18,0,0,83,84,3,2,1,0,84,90,5,19,0,0,85,86,5,5,0,0,86,87,5,18,0,0,87,88,
3,2,1,0,88,89,5,19,0,0,89,91,1,0,0,0,90,85,1,0,0,0,90,91,1,0,0,0,91,117,
1,0,0,0,92,93,5,4,0,0,93,94,5,18,0,0,94,95,3,2,1,0,95,100,5,19,0,0,96,97,
5,5,0,0,97,98,3,2,1,0,98,99,5,19,0,0,99,101,1,0,0,0,100,96,1,0,0,0,100,101,
1,0,0,0,101,117,1,0,0,0,102,103,5,11,0,0,103,104,3,2,1,0,104,105,5,12,0,
0,105,106,3,2,1,3,106,117,1,0,0,0,107,108,5,11,0,0,108,109,3,2,1,0,109,110,
5,13,0,0,110,111,3,2,1,2,111,117,1,0,0,0,112,113,5,18,0,0,113,114,3,2,1,
0,114,115,5,19,0,0,115,117,1,0,0,0,116,19,1,0,0,0,116,21,1,0,0,0,116,22,
1,0,0,0,116,24,1,0,0,0,116,38,1,0,0,0,116,40,1,0,0,0,116,41,1,0,0,0,116,
44,1,0,0,0,116,47,1,0,0,0,116,52,1,0,0,0,116,57,1,0,0,0,116,64,1,0,0,0,116,
71,1,0,0,0,116,76,1,0,0,0,116,81,1,0,0,0,116,92,1,0,0,0,116,102,1,0,0,0,
116,107,1,0,0,0,116,112,1,0,0,0,117,126,1,0,0,0,118,119,10,19,0,0,119,120,
5,9,0,0,120,125,3,2,1,20,121,122,10,18,0,0,122,123,5,8,0,0,123,125,3,2,1,
19,124,118,1,0,0,0,124,121,1,0,0,0,125,128,1,0,0,0,126,124,1,0,0,0,126,127,
1,0,0,0,127,3,1,0,0,0,128,126,1,0,0,0,129,130,5,27,0,0,130,139,5,18,0,0,
131,136,3,14,7,0,132,133,5,21,0,0,133,135,3,14,7,0,134,132,1,0,0,0,135,138,
1,0,0,0,136,134,1,0,0,0,136,137,1,0,0,0,137,140,1,0,0,0,138,136,1,0,0,0,
139,131,1,0,0,0,139,140,1,0,0,0,140,141,1,0,0,0,141,142,5,19,0,0,142,5,1,
0,0,0,143,144,3,12,6,0,144,7,1,0,0,0,145,149,3,10,5,0,146,148,5,32,0,0,147,
146,1,0,0,0,148,151,1,0,0,0,149,147,1,0,0,0,149,150,1,0,0,0,150,152,1,0,
0,0,151,149,1,0,0,0,152,153,3,12,6,0,153,9,1,0,0,0,154,155,7,0,0,0,155,11,
1,0,0,0,156,161,5,6,0,0,157,161,5,7,0,0,158,161,5,27,0,0,159,161,3,4,2,0,
160,156,1,0,0,0,160,157,1,0,0,0,160,158,1,0,0,0,160,159,1,0,0,0,161,13,1,
0,0,0,162,168,5,6,0,0,163,168,5,7,0,0,164,168,5,28,0,0,165,168,5,29,0,0,
166,168,5,31,0,0,167,162,1,0,0,0,167,163,1,0,0,0,167,164,1,0,0,0,167,165,
1,0,0,0,167,166,1,0,0,0,168,15,1,0,0,0,13,30,36,38,90,100,116,124,126,136,
139,149,160,167];


const atn = new antlr4.atn.ATNDeserializer().deserialize(serializedATN);

const decisionsToDFA = atn.decisionToState.map( (ds, index) => new antlr4.dfa.DFA(ds, index) );

const sharedContextCache = new antlr4.atn.PredictionContextCache();

export default class ModalityParser extends antlr4.Parser {

    static grammarFileName = "Modality.g4";
    static literalNames = [ null, "'must'", "'can'", "'always'", "'eventually'", 
                            "'until'", "'true'", "'false'", "'and'", "'or'", 
                            "'not'", "'when'", "'also'", "'next'", "'['", 
                            "']'", "'<'", "'>'", "'('", "')'", "'*'", "','", 
                            "'+'", "'-'", "'?'", "'lfp'", "'gfp'" ];
    static symbolicNames = [ null, "MUST", "CAN", "ALWAYS", "EVENTUALLY", 
                             "UNTIL", "TRUE", "FALSE", "AND", "OR", "NOT", 
                             "WHEN", "ALSO", "NEXT", "LBOX", "RBOX", "LDIA", 
                             "RDIA", "LPAREN", "RPAREN", "STAR", "COMMA", 
                             "PLUS", "MINUS", "QMARK", "LFP", "GFP", "NAME", 
                             "STRING", "NUMBER", "STATE_SET_VARIABLE", "VARIABLE", 
                             "WS", "LINE_COMMENT" ];
    static ruleNames = [ "expression", "formula", "functionProp", "unsignedProp", 
                         "signedProp", "sign", "prop", "arg" ];

    constructor(input) {
        super(input);
        this._interp = new antlr4.atn.ParserATNSimulator(this, atn, decisionsToDFA, sharedContextCache);
        this.ruleNames = ModalityParser.ruleNames;
        this.literalNames = ModalityParser.literalNames;
        this.symbolicNames = ModalityParser.symbolicNames;
    }

    sempred(localctx, ruleIndex, predIndex) {
    	switch(ruleIndex) {
    	case 1:
    	    		return this.formula_sempred(localctx, predIndex);
        default:
            throw "No predicate with index:" + ruleIndex;
       }
    }

    formula_sempred(localctx, predIndex) {
    	switch(predIndex) {
    		case 0:
    			return this.precpred(this._ctx, 19);
    		case 1:
    			return this.precpred(this._ctx, 18);
    		default:
    			throw "No predicate with index:" + predIndex;
    	}
    };




	expression() {
	    let localctx = new ExpressionContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 0, ModalityParser.RULE_expression);
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 16;
	        localctx.f = this.formula(0);
	        this.state = 17;
	        this.match(ModalityParser.EOF);
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


	formula(_p) {
		if(_p===undefined) {
		    _p = 0;
		}
	    const _parentctx = this._ctx;
	    const _parentState = this.state;
	    let localctx = new FormulaContext(this, this._ctx, _parentState);
	    let _prevctx = localctx;
	    const _startState = 2;
	    this.enterRecursionRule(localctx, 2, ModalityParser.RULE_formula, _p);
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 116;
	        this._errHandler.sync(this);
	        var la_ = this._interp.adaptivePredict(this._input,5,this._ctx);
	        switch(la_) {
	        case 1:
	            localctx = new TrueAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;

	            this.state = 20;
	            this.match(ModalityParser.TRUE);
	            break;

	        case 2:
	            localctx = new FalseAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 21;
	            this.match(ModalityParser.FALSE);
	            break;

	        case 3:
	            localctx = new NegatedFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 22;
	            this.match(ModalityParser.MINUS);
	            this.state = 23;
	            localctx.inner = this.formula(17);
	            break;

	        case 4:
	            localctx = new NotFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 24;
	            this.match(ModalityParser.NOT);
	            this.state = 25;
	            localctx.inner = this.formula(16);
	            break;

	        case 5:
	            localctx = new PropsSetContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 38;
	            this._errHandler.sync(this);
	            switch(this._input.LA(1)) {
	            case 6:
	            case 7:
	            case 27:
	                this.state = 26;
	                this.unsignedProp();
	                this.state = 30;
	                this._errHandler.sync(this);
	                var _alt = this._interp.adaptivePredict(this._input,0,this._ctx)
	                while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	                    if(_alt===1) {
	                        this.state = 27;
	                        this.signedProp(); 
	                    }
	                    this.state = 32;
	                    this._errHandler.sync(this);
	                    _alt = this._interp.adaptivePredict(this._input,0,this._ctx);
	                }

	                break;
	            case 22:
	            case 23:
	            case 24:
	                this.state = 34; 
	                this._errHandler.sync(this);
	                var _alt = 1;
	                do {
	                	switch (_alt) {
	                	case 1:
	                		this.state = 33;
	                		this.signedProp();
	                		break;
	                	default:
	                		throw new antlr4.error.NoViableAltException(this);
	                	}
	                	this.state = 36; 
	                	this._errHandler.sync(this);
	                	_alt = this._interp.adaptivePredict(this._input,1, this._ctx);
	                } while ( _alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER );
	                break;
	            default:
	                throw new antlr4.error.NoViableAltException(this);
	            }
	            break;

	        case 6:
	            localctx = new StateSetVariableContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 40;
	            this.match(ModalityParser.STATE_SET_VARIABLE);
	            break;

	        case 7:
	            localctx = new EmptyBoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 41;
	            this.match(ModalityParser.LBOX);
	            this.state = 42;
	            this.match(ModalityParser.RBOX);
	            this.state = 43;
	            localctx.outer = this.formula(13);
	            break;

	        case 8:
	            localctx = new EmptyDiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 44;
	            this.match(ModalityParser.LDIA);
	            this.state = 45;
	            this.match(ModalityParser.RDIA);
	            this.state = 46;
	            localctx.outer = this.formula(12);
	            break;

	        case 9:
	            localctx = new BoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 47;
	            this.match(ModalityParser.LBOX);
	            this.state = 48;
	            localctx.inner = this.formula(0);
	            this.state = 49;
	            this.match(ModalityParser.RBOX);
	            this.state = 50;
	            localctx.outer = this.formula(11);
	            break;

	        case 10:
	            localctx = new DiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 52;
	            this.match(ModalityParser.LDIA);
	            this.state = 53;
	            localctx.inner = this.formula(0);
	            this.state = 54;
	            this.match(ModalityParser.RDIA);
	            this.state = 55;
	            localctx.outer = this.formula(10);
	            break;

	        case 11:
	            localctx = new LfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 57;
	            this.match(ModalityParser.LFP);
	            this.state = 58;
	            this.match(ModalityParser.LPAREN);
	            this.state = 59;
	            localctx.stateSetVariable = this.formula(0);
	            this.state = 60;
	            this.match(ModalityParser.COMMA);
	            this.state = 61;
	            localctx.inner = this.formula(0);
	            this.state = 62;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 12:
	            localctx = new GfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 64;
	            this.match(ModalityParser.GFP);
	            this.state = 65;
	            this.match(ModalityParser.LPAREN);
	            this.state = 66;
	            localctx.stateSetVariable = this.formula(0);
	            this.state = 67;
	            this.match(ModalityParser.COMMA);
	            this.state = 68;
	            localctx.inner = this.formula(0);
	            this.state = 69;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 13:
	            localctx = new MustMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 71;
	            this.match(ModalityParser.MUST);
	            this.state = 72;
	            this.match(ModalityParser.LPAREN);
	            this.state = 73;
	            this.formula(0);
	            this.state = 74;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 14:
	            localctx = new CanMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 76;
	            this.match(ModalityParser.CAN);
	            this.state = 77;
	            this.match(ModalityParser.LPAREN);
	            this.state = 78;
	            this.formula(0);
	            this.state = 79;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 15:
	            localctx = new AlwaysMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 81;
	            this.match(ModalityParser.ALWAYS);
	            this.state = 82;
	            this.match(ModalityParser.LPAREN);
	            this.state = 83;
	            localctx.inner_formula = this.formula(0);
	            this.state = 84;
	            this.match(ModalityParser.RPAREN);
	            this.state = 90;
	            this._errHandler.sync(this);
	            var la_ = this._interp.adaptivePredict(this._input,3,this._ctx);
	            if(la_===1) {
	                this.state = 85;
	                this.match(ModalityParser.UNTIL);
	                this.state = 86;
	                this.match(ModalityParser.LPAREN);
	                this.state = 87;
	                localctx.until_formula = this.formula(0);
	                this.state = 88;
	                this.match(ModalityParser.RPAREN);

	            }
	            break;

	        case 16:
	            localctx = new EventuallyMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 92;
	            this.match(ModalityParser.EVENTUALLY);
	            this.state = 93;
	            this.match(ModalityParser.LPAREN);
	            this.state = 94;
	            localctx.inner_formula = this.formula(0);
	            this.state = 95;
	            this.match(ModalityParser.RPAREN);
	            this.state = 100;
	            this._errHandler.sync(this);
	            var la_ = this._interp.adaptivePredict(this._input,4,this._ctx);
	            if(la_===1) {
	                this.state = 96;
	                this.match(ModalityParser.UNTIL);
	                this.state = 97;
	                localctx.until_formula = this.formula(0);
	                this.state = 98;
	                this.match(ModalityParser.RPAREN);

	            }
	            break;

	        case 17:
	            localctx = new WhenAlsoFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 102;
	            this.match(ModalityParser.WHEN);
	            this.state = 103;
	            localctx.when_formula = this.formula(0);
	            this.state = 104;
	            this.match(ModalityParser.ALSO);
	            this.state = 105;
	            localctx.also_formula = this.formula(3);
	            break;

	        case 18:
	            localctx = new WhenNextFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 107;
	            this.match(ModalityParser.WHEN);
	            this.state = 108;
	            localctx.when_formula = this.formula(0);
	            this.state = 109;
	            this.match(ModalityParser.NEXT);
	            this.state = 110;
	            localctx.next_formula = this.formula(2);
	            break;

	        case 19:
	            localctx = new ParenFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 112;
	            this.match(ModalityParser.LPAREN);
	            this.state = 113;
	            localctx.inner = this.formula(0);
	            this.state = 114;
	            this.match(ModalityParser.RPAREN);
	            break;

	        }
	        this._ctx.stop = this._input.LT(-1);
	        this.state = 126;
	        this._errHandler.sync(this);
	        var _alt = this._interp.adaptivePredict(this._input,7,this._ctx)
	        while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	            if(_alt===1) {
	                if(this._parseListeners!==null) {
	                    this.triggerExitRuleEvent();
	                }
	                _prevctx = localctx;
	                this.state = 124;
	                this._errHandler.sync(this);
	                var la_ = this._interp.adaptivePredict(this._input,6,this._ctx);
	                switch(la_) {
	                case 1:
	                    localctx = new OrFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 118;
	                    if (!( this.precpred(this._ctx, 19))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 19)");
	                    }
	                    this.state = 119;
	                    this.match(ModalityParser.OR);
	                    this.state = 120;
	                    localctx.right = this.formula(20);
	                    break;

	                case 2:
	                    localctx = new AndFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 121;
	                    if (!( this.precpred(this._ctx, 18))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 18)");
	                    }
	                    this.state = 122;
	                    this.match(ModalityParser.AND);
	                    this.state = 123;
	                    localctx.right = this.formula(19);
	                    break;

	                } 
	            }
	            this.state = 128;
	            this._errHandler.sync(this);
	            _alt = this._interp.adaptivePredict(this._input,7,this._ctx);
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



	functionProp() {
	    let localctx = new FunctionPropContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 4, ModalityParser.RULE_functionProp);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 129;
	        localctx.name = this.match(ModalityParser.NAME);
	        this.state = 130;
	        this.match(ModalityParser.LPAREN);
	        this.state = 139;
	        this._errHandler.sync(this);
	        _la = this._input.LA(1);
	        if((((_la) & ~0x1f) === 0 && ((1 << _la) & 2952790208) !== 0)) {
	            this.state = 131;
	            this.arg();
	            this.state = 136;
	            this._errHandler.sync(this);
	            _la = this._input.LA(1);
	            while(_la===21) {
	                this.state = 132;
	                this.match(ModalityParser.COMMA);
	                this.state = 133;
	                this.arg();
	                this.state = 138;
	                this._errHandler.sync(this);
	                _la = this._input.LA(1);
	            }
	        }

	        this.state = 141;
	        this.match(ModalityParser.RPAREN);
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



	unsignedProp() {
	    let localctx = new UnsignedPropContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 6, ModalityParser.RULE_unsignedProp);
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 143;
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
	    this.enterRule(localctx, 8, ModalityParser.RULE_signedProp);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 145;
	        localctx.theSign = this.sign();
	        this.state = 149;
	        this._errHandler.sync(this);
	        _la = this._input.LA(1);
	        while(_la===32) {
	            this.state = 146;
	            this.match(ModalityParser.WS);
	            this.state = 151;
	            this._errHandler.sync(this);
	            _la = this._input.LA(1);
	        }
	        this.state = 152;
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
	    this.enterRule(localctx, 10, ModalityParser.RULE_sign);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 154;
	        _la = this._input.LA(1);
	        if(!((((_la) & ~0x1f) === 0 && ((1 << _la) & 29360128) !== 0))) {
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
	    this.enterRule(localctx, 12, ModalityParser.RULE_prop);
	    try {
	        this.state = 160;
	        this._errHandler.sync(this);
	        var la_ = this._interp.adaptivePredict(this._input,11,this._ctx);
	        switch(la_) {
	        case 1:
	            this.enterOuterAlt(localctx, 1);
	            this.state = 156;
	            this.match(ModalityParser.TRUE);
	            break;

	        case 2:
	            this.enterOuterAlt(localctx, 2);
	            this.state = 157;
	            this.match(ModalityParser.FALSE);
	            break;

	        case 3:
	            this.enterOuterAlt(localctx, 3);
	            this.state = 158;
	            this.match(ModalityParser.NAME);
	            break;

	        case 4:
	            this.enterOuterAlt(localctx, 4);
	            this.state = 159;
	            this.functionProp();
	            break;

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



	arg() {
	    let localctx = new ArgContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 14, ModalityParser.RULE_arg);
	    try {
	        this.state = 167;
	        this._errHandler.sync(this);
	        switch(this._input.LA(1)) {
	        case 6:
	            localctx = new TrueArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 1);
	            this.state = 162;
	            this.match(ModalityParser.TRUE);
	            break;
	        case 7:
	            localctx = new FalseArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 2);
	            this.state = 163;
	            this.match(ModalityParser.FALSE);
	            break;
	        case 28:
	            localctx = new StringArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 3);
	            this.state = 164;
	            this.match(ModalityParser.STRING);
	            break;
	        case 29:
	            localctx = new NumberArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 4);
	            this.state = 165;
	            this.match(ModalityParser.NUMBER);
	            break;
	        case 31:
	            localctx = new VariableArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 5);
	            this.state = 166;
	            this.match(ModalityParser.VARIABLE);
	            break;
	        default:
	            throw new antlr4.error.NoViableAltException(this);
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

ModalityParser.EOF = antlr4.Token.EOF;
ModalityParser.MUST = 1;
ModalityParser.CAN = 2;
ModalityParser.ALWAYS = 3;
ModalityParser.EVENTUALLY = 4;
ModalityParser.UNTIL = 5;
ModalityParser.TRUE = 6;
ModalityParser.FALSE = 7;
ModalityParser.AND = 8;
ModalityParser.OR = 9;
ModalityParser.NOT = 10;
ModalityParser.WHEN = 11;
ModalityParser.ALSO = 12;
ModalityParser.NEXT = 13;
ModalityParser.LBOX = 14;
ModalityParser.RBOX = 15;
ModalityParser.LDIA = 16;
ModalityParser.RDIA = 17;
ModalityParser.LPAREN = 18;
ModalityParser.RPAREN = 19;
ModalityParser.STAR = 20;
ModalityParser.COMMA = 21;
ModalityParser.PLUS = 22;
ModalityParser.MINUS = 23;
ModalityParser.QMARK = 24;
ModalityParser.LFP = 25;
ModalityParser.GFP = 26;
ModalityParser.NAME = 27;
ModalityParser.STRING = 28;
ModalityParser.NUMBER = 29;
ModalityParser.STATE_SET_VARIABLE = 30;
ModalityParser.VARIABLE = 31;
ModalityParser.WS = 32;
ModalityParser.LINE_COMMENT = 33;

ModalityParser.RULE_expression = 0;
ModalityParser.RULE_formula = 1;
ModalityParser.RULE_functionProp = 2;
ModalityParser.RULE_unsignedProp = 3;
ModalityParser.RULE_signedProp = 4;
ModalityParser.RULE_sign = 5;
ModalityParser.RULE_prop = 6;
ModalityParser.RULE_arg = 7;

class ExpressionContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalityParser.RULE_expression;
        this.f = null;
    }

	EOF() {
	    return this.getToken(ModalityParser.EOF, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterExpression(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitExpression(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitExpression(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



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
        this.ruleIndex = ModalityParser.RULE_formula;
    }


	 
		copyFrom(ctx) {
			super.copyFrom(ctx);
		}

}


class PropsSetContext extends FormulaContext {

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
	    if(listener instanceof ModalityListener ) {
	        listener.enterPropsSet(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitPropsSet(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitPropsSet(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.PropsSetContext = PropsSetContext;

class WhenAlsoFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.when_formula = null;;
        this.also_formula = null;;
        super.copyFrom(ctx);
    }

	WHEN() {
	    return this.getToken(ModalityParser.WHEN, 0);
	};

	ALSO() {
	    return this.getToken(ModalityParser.ALSO, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterWhenAlsoFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitWhenAlsoFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitWhenAlsoFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.WhenAlsoFormulaContext = WhenAlsoFormulaContext;

class EmptyBoxFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LBOX() {
	    return this.getToken(ModalityParser.LBOX, 0);
	};

	RBOX() {
	    return this.getToken(ModalityParser.RBOX, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterEmptyBoxFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitEmptyBoxFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitEmptyBoxFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.EmptyBoxFormulaContext = EmptyBoxFormulaContext;

class AlwaysMacroContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner_formula = null;;
        this.until_formula = null;;
        super.copyFrom(ctx);
    }

	ALWAYS() {
	    return this.getToken(ModalityParser.ALWAYS, 0);
	};

	LPAREN = function(i) {
		if(i===undefined) {
			i = null;
		}
	    if(i===null) {
	        return this.getTokens(ModalityParser.LPAREN);
	    } else {
	        return this.getToken(ModalityParser.LPAREN, i);
	    }
	};


	RPAREN = function(i) {
		if(i===undefined) {
			i = null;
		}
	    if(i===null) {
	        return this.getTokens(ModalityParser.RPAREN);
	    } else {
	        return this.getToken(ModalityParser.RPAREN, i);
	    }
	};


	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	UNTIL() {
	    return this.getToken(ModalityParser.UNTIL, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterAlwaysMacro(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitAlwaysMacro(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitAlwaysMacro(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.AlwaysMacroContext = AlwaysMacroContext;

class WhenNextFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.when_formula = null;;
        this.next_formula = null;;
        super.copyFrom(ctx);
    }

	WHEN() {
	    return this.getToken(ModalityParser.WHEN, 0);
	};

	NEXT() {
	    return this.getToken(ModalityParser.NEXT, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterWhenNextFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitWhenNextFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitWhenNextFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.WhenNextFormulaContext = WhenNextFormulaContext;

class EmptyDiamondFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LDIA() {
	    return this.getToken(ModalityParser.LDIA, 0);
	};

	RDIA() {
	    return this.getToken(ModalityParser.RDIA, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterEmptyDiamondFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitEmptyDiamondFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitEmptyDiamondFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.EmptyDiamondFormulaContext = EmptyDiamondFormulaContext;

class AndFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.left = null;;
        this.right = null;;
        super.copyFrom(ctx);
    }

	AND() {
	    return this.getToken(ModalityParser.AND, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterAndFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitAndFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitAndFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.AndFormulaContext = AndFormulaContext;

class NotFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        super.copyFrom(ctx);
    }

	NOT() {
	    return this.getToken(ModalityParser.NOT, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterNotFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitNotFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitNotFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.NotFormulaContext = NotFormulaContext;

class OrFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.left = null;;
        this.right = null;;
        super.copyFrom(ctx);
    }

	OR() {
	    return this.getToken(ModalityParser.OR, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterOrFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitOrFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitOrFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.OrFormulaContext = OrFormulaContext;

class LfpFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.stateSetVariable = null;;
        this.inner = null;;
        super.copyFrom(ctx);
    }

	LFP() {
	    return this.getToken(ModalityParser.LFP, 0);
	};

	LPAREN() {
	    return this.getToken(ModalityParser.LPAREN, 0);
	};

	COMMA() {
	    return this.getToken(ModalityParser.COMMA, 0);
	};

	RPAREN() {
	    return this.getToken(ModalityParser.RPAREN, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterLfpFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitLfpFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitLfpFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.LfpFormulaContext = LfpFormulaContext;

class CanMacroContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	CAN() {
	    return this.getToken(ModalityParser.CAN, 0);
	};

	LPAREN() {
	    return this.getToken(ModalityParser.LPAREN, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	RPAREN() {
	    return this.getToken(ModalityParser.RPAREN, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterCanMacro(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitCanMacro(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitCanMacro(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.CanMacroContext = CanMacroContext;

class EventuallyMacroContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner_formula = null;;
        this.until_formula = null;;
        super.copyFrom(ctx);
    }

	EVENTUALLY() {
	    return this.getToken(ModalityParser.EVENTUALLY, 0);
	};

	LPAREN() {
	    return this.getToken(ModalityParser.LPAREN, 0);
	};

	RPAREN = function(i) {
		if(i===undefined) {
			i = null;
		}
	    if(i===null) {
	        return this.getTokens(ModalityParser.RPAREN);
	    } else {
	        return this.getToken(ModalityParser.RPAREN, i);
	    }
	};


	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	UNTIL() {
	    return this.getToken(ModalityParser.UNTIL, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterEventuallyMacro(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitEventuallyMacro(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitEventuallyMacro(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.EventuallyMacroContext = EventuallyMacroContext;

class DiamondFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LDIA() {
	    return this.getToken(ModalityParser.LDIA, 0);
	};

	RDIA() {
	    return this.getToken(ModalityParser.RDIA, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterDiamondFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitDiamondFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitDiamondFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.DiamondFormulaContext = DiamondFormulaContext;

class ParenFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        super.copyFrom(ctx);
    }

	LPAREN() {
	    return this.getToken(ModalityParser.LPAREN, 0);
	};

	RPAREN() {
	    return this.getToken(ModalityParser.RPAREN, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterParenFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitParenFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitParenFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.ParenFormulaContext = ParenFormulaContext;

class BoxFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        this.outer = null;;
        super.copyFrom(ctx);
    }

	LBOX() {
	    return this.getToken(ModalityParser.LBOX, 0);
	};

	RBOX() {
	    return this.getToken(ModalityParser.RBOX, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterBoxFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitBoxFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitBoxFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.BoxFormulaContext = BoxFormulaContext;

class MustMacroContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	MUST() {
	    return this.getToken(ModalityParser.MUST, 0);
	};

	LPAREN() {
	    return this.getToken(ModalityParser.LPAREN, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	RPAREN() {
	    return this.getToken(ModalityParser.RPAREN, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterMustMacro(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitMustMacro(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitMustMacro(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.MustMacroContext = MustMacroContext;

class FalseAtomContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	FALSE() {
	    return this.getToken(ModalityParser.FALSE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterFalseAtom(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitFalseAtom(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitFalseAtom(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.FalseAtomContext = FalseAtomContext;

class NegatedFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        super.copyFrom(ctx);
    }

	MINUS() {
	    return this.getToken(ModalityParser.MINUS, 0);
	};

	formula() {
	    return this.getTypedRuleContext(FormulaContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterNegatedFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitNegatedFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitNegatedFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.NegatedFormulaContext = NegatedFormulaContext;

class GfpFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.stateSetVariable = null;;
        this.inner = null;;
        super.copyFrom(ctx);
    }

	GFP() {
	    return this.getToken(ModalityParser.GFP, 0);
	};

	LPAREN() {
	    return this.getToken(ModalityParser.LPAREN, 0);
	};

	COMMA() {
	    return this.getToken(ModalityParser.COMMA, 0);
	};

	RPAREN() {
	    return this.getToken(ModalityParser.RPAREN, 0);
	};

	formula = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(FormulaContext);
	    } else {
	        return this.getTypedRuleContext(FormulaContext,i);
	    }
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterGfpFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitGfpFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitGfpFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.GfpFormulaContext = GfpFormulaContext;

class TrueAtomContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	TRUE() {
	    return this.getToken(ModalityParser.TRUE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterTrueAtom(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitTrueAtom(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitTrueAtom(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.TrueAtomContext = TrueAtomContext;

class StateSetVariableContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	STATE_SET_VARIABLE() {
	    return this.getToken(ModalityParser.STATE_SET_VARIABLE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterStateSetVariable(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitStateSetVariable(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitStateSetVariable(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.StateSetVariableContext = StateSetVariableContext;

class FunctionPropContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalityParser.RULE_functionProp;
        this.name = null;
    }

	LPAREN() {
	    return this.getToken(ModalityParser.LPAREN, 0);
	};

	RPAREN() {
	    return this.getToken(ModalityParser.RPAREN, 0);
	};

	NAME() {
	    return this.getToken(ModalityParser.NAME, 0);
	};

	arg = function(i) {
	    if(i===undefined) {
	        i = null;
	    }
	    if(i===null) {
	        return this.getTypedRuleContexts(ArgContext);
	    } else {
	        return this.getTypedRuleContext(ArgContext,i);
	    }
	};

	COMMA = function(i) {
		if(i===undefined) {
			i = null;
		}
	    if(i===null) {
	        return this.getTokens(ModalityParser.COMMA);
	    } else {
	        return this.getToken(ModalityParser.COMMA, i);
	    }
	};


	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterFunctionProp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitFunctionProp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitFunctionProp(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



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
        this.ruleIndex = ModalityParser.RULE_unsignedProp;
        this.theProp = null;
    }

	prop() {
	    return this.getTypedRuleContext(PropContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterUnsignedProp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitUnsignedProp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
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
        this.ruleIndex = ModalityParser.RULE_signedProp;
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
	        return this.getTokens(ModalityParser.WS);
	    } else {
	        return this.getToken(ModalityParser.WS, i);
	    }
	};


	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterSignedProp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitSignedProp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
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
        this.ruleIndex = ModalityParser.RULE_sign;
    }

	PLUS() {
	    return this.getToken(ModalityParser.PLUS, 0);
	};

	MINUS() {
	    return this.getToken(ModalityParser.MINUS, 0);
	};

	QMARK() {
	    return this.getToken(ModalityParser.QMARK, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterSign(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitSign(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
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
        this.ruleIndex = ModalityParser.RULE_prop;
    }

	TRUE() {
	    return this.getToken(ModalityParser.TRUE, 0);
	};

	FALSE() {
	    return this.getToken(ModalityParser.FALSE, 0);
	};

	NAME() {
	    return this.getToken(ModalityParser.NAME, 0);
	};

	functionProp() {
	    return this.getTypedRuleContext(FunctionPropContext,0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterProp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitProp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitProp(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



class ArgContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalityParser.RULE_arg;
    }


	 
		copyFrom(ctx) {
			super.copyFrom(ctx);
		}

}


class FalseArgContext extends ArgContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	FALSE() {
	    return this.getToken(ModalityParser.FALSE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterFalseArg(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitFalseArg(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitFalseArg(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.FalseArgContext = FalseArgContext;

class VariableArgContext extends ArgContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	VARIABLE() {
	    return this.getToken(ModalityParser.VARIABLE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterVariableArg(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitVariableArg(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitVariableArg(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.VariableArgContext = VariableArgContext;

class TrueArgContext extends ArgContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	TRUE() {
	    return this.getToken(ModalityParser.TRUE, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterTrueArg(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitTrueArg(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitTrueArg(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.TrueArgContext = TrueArgContext;

class StringArgContext extends ArgContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	STRING() {
	    return this.getToken(ModalityParser.STRING, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterStringArg(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitStringArg(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitStringArg(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.StringArgContext = StringArgContext;

class NumberArgContext extends ArgContext {

    constructor(parser, ctx) {
        super(parser);
        super.copyFrom(ctx);
    }

	NUMBER() {
	    return this.getToken(ModalityParser.NUMBER, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterNumberArg(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitNumberArg(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitNumberArg(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.NumberArgContext = NumberArgContext;


ModalityParser.ExpressionContext = ExpressionContext; 
ModalityParser.FormulaContext = FormulaContext; 
ModalityParser.FunctionPropContext = FunctionPropContext; 
ModalityParser.UnsignedPropContext = UnsignedPropContext; 
ModalityParser.SignedPropContext = SignedPropContext; 
ModalityParser.SignContext = SignContext; 
ModalityParser.PropContext = PropContext; 
ModalityParser.ArgContext = ArgContext; 
