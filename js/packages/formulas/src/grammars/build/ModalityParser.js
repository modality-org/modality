// jshint ignore: start
import antlr4 from 'antlr4';
import ModalityListener from './ModalityListener.js';
import ModalityVisitor from './ModalityVisitor.js';

const serializedATN = [4,1,37,203,2,0,7,0,2,1,7,1,2,2,7,2,2,3,7,3,2,4,7,
4,2,5,7,5,2,6,7,6,2,7,7,7,2,8,7,8,2,9,7,9,1,0,1,0,1,0,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,5,1,34,8,1,10,1,12,1,37,9,1,1,1,4,1,40,8,1,11,1,12,
1,41,3,1,44,8,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,1,108,8,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,3,1,119,8,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,1,142,8,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,5,1,154,8,1,10,1,12,1,157,9,1,1,2,1,2,1,2,1,
2,1,2,5,2,164,8,2,10,2,12,2,167,9,2,3,2,169,8,2,1,2,1,2,1,3,1,3,1,4,1,4,
5,4,177,8,4,10,4,12,4,180,9,4,1,4,1,4,1,5,1,5,1,6,1,6,1,6,1,6,3,6,190,8,
6,1,7,1,7,1,8,1,8,1,9,1,9,1,9,1,9,1,9,3,9,201,8,9,1,9,0,1,2,10,0,2,4,6,8,
10,12,14,16,18,0,3,1,0,23,25,2,0,11,11,26,27,2,0,10,10,35,35,231,0,20,1,
0,0,0,2,141,1,0,0,0,4,158,1,0,0,0,6,172,1,0,0,0,8,174,1,0,0,0,10,183,1,0,
0,0,12,189,1,0,0,0,14,191,1,0,0,0,16,193,1,0,0,0,18,200,1,0,0,0,20,21,3,
2,1,0,21,22,5,0,0,1,22,1,1,0,0,0,23,24,6,1,-1,0,24,142,5,6,0,0,25,142,5,
7,0,0,26,27,5,24,0,0,27,142,3,2,1,20,28,29,3,16,8,0,29,30,3,2,1,19,30,142,
1,0,0,0,31,35,3,6,3,0,32,34,3,8,4,0,33,32,1,0,0,0,34,37,1,0,0,0,35,33,1,
0,0,0,35,36,1,0,0,0,36,44,1,0,0,0,37,35,1,0,0,0,38,40,3,8,4,0,39,38,1,0,
0,0,40,41,1,0,0,0,41,39,1,0,0,0,41,42,1,0,0,0,42,44,1,0,0,0,43,31,1,0,0,
0,43,39,1,0,0,0,44,142,1,0,0,0,45,142,5,33,0,0,46,47,5,15,0,0,47,48,5,16,
0,0,48,142,3,2,1,16,49,50,5,17,0,0,50,51,5,18,0,0,51,142,3,2,1,15,52,53,
5,15,0,0,53,54,3,2,1,0,54,55,5,16,0,0,55,56,3,14,7,0,56,57,3,2,1,0,57,142,
1,0,0,0,58,59,5,17,0,0,59,60,3,2,1,0,60,61,5,18,0,0,61,62,3,14,7,0,62,63,
3,2,1,0,63,142,1,0,0,0,64,65,5,15,0,0,65,66,3,2,1,0,66,67,5,16,0,0,67,68,
3,2,1,0,68,142,1,0,0,0,69,70,5,17,0,0,70,71,3,2,1,0,71,72,5,18,0,0,72,73,
3,2,1,0,73,142,1,0,0,0,74,75,5,28,0,0,75,76,5,19,0,0,76,77,3,2,1,0,77,78,
5,22,0,0,78,79,3,2,1,0,79,80,5,20,0,0,80,142,1,0,0,0,81,82,5,29,0,0,82,83,
5,19,0,0,83,84,3,2,1,0,84,85,5,22,0,0,85,86,3,2,1,0,86,87,5,20,0,0,87,142,
1,0,0,0,88,89,5,1,0,0,89,90,5,19,0,0,90,91,3,2,1,0,91,92,5,20,0,0,92,142,
1,0,0,0,93,94,5,2,0,0,94,95,5,19,0,0,95,96,3,2,1,0,96,97,5,20,0,0,97,142,
1,0,0,0,98,99,5,3,0,0,99,100,5,19,0,0,100,101,3,2,1,0,101,107,5,20,0,0,102,
103,5,5,0,0,103,104,5,19,0,0,104,105,3,2,1,0,105,106,5,20,0,0,106,108,1,
0,0,0,107,102,1,0,0,0,107,108,1,0,0,0,108,142,1,0,0,0,109,110,5,4,0,0,110,
111,5,19,0,0,111,112,3,2,1,0,112,118,5,20,0,0,113,114,5,5,0,0,114,115,5,
19,0,0,115,116,3,2,1,0,116,117,5,20,0,0,117,119,1,0,0,0,118,113,1,0,0,0,
118,119,1,0,0,0,119,142,1,0,0,0,120,121,5,5,0,0,121,122,5,19,0,0,122,123,
3,2,1,0,123,124,5,22,0,0,124,125,3,2,1,0,125,126,5,20,0,0,126,142,1,0,0,
0,127,128,5,12,0,0,128,129,3,2,1,0,129,130,5,13,0,0,130,131,3,2,1,0,131,
142,1,0,0,0,132,133,5,12,0,0,133,134,3,2,1,0,134,135,5,14,0,0,135,136,3,
2,1,0,136,142,1,0,0,0,137,138,5,19,0,0,138,139,3,2,1,0,139,140,5,20,0,0,
140,142,1,0,0,0,141,23,1,0,0,0,141,25,1,0,0,0,141,26,1,0,0,0,141,28,1,0,
0,0,141,43,1,0,0,0,141,45,1,0,0,0,141,46,1,0,0,0,141,49,1,0,0,0,141,52,1,
0,0,0,141,58,1,0,0,0,141,64,1,0,0,0,141,69,1,0,0,0,141,74,1,0,0,0,141,81,
1,0,0,0,141,88,1,0,0,0,141,93,1,0,0,0,141,98,1,0,0,0,141,109,1,0,0,0,141,
120,1,0,0,0,141,127,1,0,0,0,141,132,1,0,0,0,141,137,1,0,0,0,142,155,1,0,
0,0,143,144,10,23,0,0,144,145,5,9,0,0,145,154,3,2,1,24,146,147,10,22,0,0,
147,148,5,8,0,0,148,154,3,2,1,23,149,150,10,21,0,0,150,151,3,14,7,0,151,
152,3,2,1,22,152,154,1,0,0,0,153,143,1,0,0,0,153,146,1,0,0,0,153,149,1,0,
0,0,154,157,1,0,0,0,155,153,1,0,0,0,155,156,1,0,0,0,156,3,1,0,0,0,157,155,
1,0,0,0,158,159,5,30,0,0,159,168,5,19,0,0,160,165,3,18,9,0,161,162,5,22,
0,0,162,164,3,18,9,0,163,161,1,0,0,0,164,167,1,0,0,0,165,163,1,0,0,0,165,
166,1,0,0,0,166,169,1,0,0,0,167,165,1,0,0,0,168,160,1,0,0,0,168,169,1,0,
0,0,169,170,1,0,0,0,170,171,5,20,0,0,171,5,1,0,0,0,172,173,3,12,6,0,173,
7,1,0,0,0,174,178,3,10,5,0,175,177,5,36,0,0,176,175,1,0,0,0,177,180,1,0,
0,0,178,176,1,0,0,0,178,179,1,0,0,0,179,181,1,0,0,0,180,178,1,0,0,0,181,
182,3,12,6,0,182,9,1,0,0,0,183,184,7,0,0,0,184,11,1,0,0,0,185,190,5,6,0,
0,186,190,5,7,0,0,187,190,5,30,0,0,188,190,3,4,2,0,189,185,1,0,0,0,189,186,
1,0,0,0,189,187,1,0,0,0,189,188,1,0,0,0,190,13,1,0,0,0,191,192,7,1,0,0,192,
15,1,0,0,0,193,194,7,2,0,0,194,17,1,0,0,0,195,201,5,6,0,0,196,201,5,7,0,
0,197,201,5,31,0,0,198,201,5,32,0,0,199,201,5,34,0,0,200,195,1,0,0,0,200,
196,1,0,0,0,200,197,1,0,0,0,200,198,1,0,0,0,200,199,1,0,0,0,201,19,1,0,0,
0,13,35,41,43,107,118,141,153,155,165,168,178,189,200];


const atn = new antlr4.atn.ATNDeserializer().deserialize(serializedATN);

const decisionsToDFA = atn.decisionToState.map( (ds, index) => new antlr4.dfa.DFA(ds, index) );

const sharedContextCache = new antlr4.atn.PredictionContextCache();

export default class ModalityParser extends antlr4.Parser {

    static grammarFileName = "Modality.g4";
    static literalNames = [ null, "'must'", "'can'", "'always'", "'eventually'", 
                            "'until'", "'true'", "'false'", "'and'", "'or'", 
                            "'not'", "'implies'", "'when'", "'also'", "'next'", 
                            "'['", "']'", "'<'", "'>'", "'('", "')'", "'*'", 
                            "','", "'+'", "'-'", "'?'", "'->'", "'=>'", 
                            "'lfp'", "'gfp'", null, null, null, null, null, 
                            "'!'" ];
    static symbolicNames = [ null, "MUST", "CAN", "ALWAYS", "EVENTUALLY", 
                             "UNTIL", "TRUE", "FALSE", "AND", "OR", "NOT", 
                             "IMPLIES", "WHEN", "ALSO", "NEXT", "LBOX", 
                             "RBOX", "LDIA", "RDIA", "LPAREN", "RPAREN", 
                             "STAR", "COMMA", "PLUS", "MINUS", "QMARK", 
                             "ARROW", "FAT_ARROW", "LFP", "GFP", "NAME", 
                             "STRING", "NUMBER", "STATE_SET_VARIABLE", "VARIABLE", 
                             "BANG", "WS", "LINE_COMMENT" ];
    static ruleNames = [ "expression", "formula", "functionProp", "unsignedProp", 
                         "signedProp", "sign", "prop", "implication", "notOp", 
                         "arg" ];

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
    			return this.precpred(this._ctx, 23);
    		case 1:
    			return this.precpred(this._ctx, 22);
    		case 2:
    			return this.precpred(this._ctx, 21);
    		default:
    			throw "No predicate with index:" + predIndex;
    	}
    };




	expression() {
	    let localctx = new ExpressionContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 0, ModalityParser.RULE_expression);
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 20;
	        localctx.f = this.formula(0);
	        this.state = 21;
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
	        this.state = 141;
	        this._errHandler.sync(this);
	        var la_ = this._interp.adaptivePredict(this._input,5,this._ctx);
	        switch(la_) {
	        case 1:
	            localctx = new TrueAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;

	            this.state = 24;
	            this.match(ModalityParser.TRUE);
	            break;

	        case 2:
	            localctx = new FalseAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 25;
	            this.match(ModalityParser.FALSE);
	            break;

	        case 3:
	            localctx = new NegatedFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 26;
	            this.match(ModalityParser.MINUS);
	            this.state = 27;
	            localctx.inner = this.formula(20);
	            break;

	        case 4:
	            localctx = new NotFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 28;
	            this.notOp();
	            this.state = 29;
	            localctx.inner = this.formula(19);
	            break;

	        case 5:
	            localctx = new PropsSetContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 43;
	            this._errHandler.sync(this);
	            switch(this._input.LA(1)) {
	            case 6:
	            case 7:
	            case 30:
	                this.state = 31;
	                this.unsignedProp();
	                this.state = 35;
	                this._errHandler.sync(this);
	                var _alt = this._interp.adaptivePredict(this._input,0,this._ctx)
	                while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	                    if(_alt===1) {
	                        this.state = 32;
	                        this.signedProp(); 
	                    }
	                    this.state = 37;
	                    this._errHandler.sync(this);
	                    _alt = this._interp.adaptivePredict(this._input,0,this._ctx);
	                }

	                break;
	            case 23:
	            case 24:
	            case 25:
	                this.state = 39; 
	                this._errHandler.sync(this);
	                var _alt = 1;
	                do {
	                	switch (_alt) {
	                	case 1:
	                		this.state = 38;
	                		this.signedProp();
	                		break;
	                	default:
	                		throw new antlr4.error.NoViableAltException(this);
	                	}
	                	this.state = 41; 
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
	            this.state = 45;
	            this.match(ModalityParser.STATE_SET_VARIABLE);
	            break;

	        case 7:
	            localctx = new EmptyBoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 46;
	            this.match(ModalityParser.LBOX);
	            this.state = 47;
	            this.match(ModalityParser.RBOX);
	            this.state = 48;
	            localctx.outer = this.formula(16);
	            break;

	        case 8:
	            localctx = new EmptyDiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 49;
	            this.match(ModalityParser.LDIA);
	            this.state = 50;
	            this.match(ModalityParser.RDIA);
	            this.state = 51;
	            localctx.outer = this.formula(15);
	            break;

	        case 9:
	            localctx = new BoxGuardImpliesFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 52;
	            this.match(ModalityParser.LBOX);
	            this.state = 53;
	            localctx.inner = this.formula(0);
	            this.state = 54;
	            this.match(ModalityParser.RBOX);
	            this.state = 55;
	            this.implication();
	            this.state = 56;
	            localctx.right = this.formula(0);
	            break;

	        case 10:
	            localctx = new DiamondGuardImpliesFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 58;
	            this.match(ModalityParser.LDIA);
	            this.state = 59;
	            localctx.inner = this.formula(0);
	            this.state = 60;
	            this.match(ModalityParser.RDIA);
	            this.state = 61;
	            this.implication();
	            this.state = 62;
	            localctx.right = this.formula(0);
	            break;

	        case 11:
	            localctx = new BoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 64;
	            this.match(ModalityParser.LBOX);
	            this.state = 65;
	            localctx.inner = this.formula(0);
	            this.state = 66;
	            this.match(ModalityParser.RBOX);
	            this.state = 67;
	            localctx.outer = this.formula(0);
	            break;

	        case 12:
	            localctx = new DiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 69;
	            this.match(ModalityParser.LDIA);
	            this.state = 70;
	            localctx.inner = this.formula(0);
	            this.state = 71;
	            this.match(ModalityParser.RDIA);
	            this.state = 72;
	            localctx.outer = this.formula(0);
	            break;

	        case 13:
	            localctx = new LfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 74;
	            this.match(ModalityParser.LFP);
	            this.state = 75;
	            this.match(ModalityParser.LPAREN);
	            this.state = 76;
	            localctx.stateSetVariable = this.formula(0);
	            this.state = 77;
	            this.match(ModalityParser.COMMA);
	            this.state = 78;
	            localctx.inner = this.formula(0);
	            this.state = 79;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 14:
	            localctx = new GfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 81;
	            this.match(ModalityParser.GFP);
	            this.state = 82;
	            this.match(ModalityParser.LPAREN);
	            this.state = 83;
	            localctx.stateSetVariable = this.formula(0);
	            this.state = 84;
	            this.match(ModalityParser.COMMA);
	            this.state = 85;
	            localctx.inner = this.formula(0);
	            this.state = 86;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 15:
	            localctx = new MustMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 88;
	            this.match(ModalityParser.MUST);
	            this.state = 89;
	            this.match(ModalityParser.LPAREN);
	            this.state = 90;
	            this.formula(0);
	            this.state = 91;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 16:
	            localctx = new CanMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 93;
	            this.match(ModalityParser.CAN);
	            this.state = 94;
	            this.match(ModalityParser.LPAREN);
	            this.state = 95;
	            this.formula(0);
	            this.state = 96;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 17:
	            localctx = new AlwaysMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 98;
	            this.match(ModalityParser.ALWAYS);
	            this.state = 99;
	            this.match(ModalityParser.LPAREN);
	            this.state = 100;
	            localctx.inner_formula = this.formula(0);
	            this.state = 101;
	            this.match(ModalityParser.RPAREN);
	            this.state = 107;
	            this._errHandler.sync(this);
	            var la_ = this._interp.adaptivePredict(this._input,3,this._ctx);
	            if(la_===1) {
	                this.state = 102;
	                this.match(ModalityParser.UNTIL);
	                this.state = 103;
	                this.match(ModalityParser.LPAREN);
	                this.state = 104;
	                localctx.until_formula = this.formula(0);
	                this.state = 105;
	                this.match(ModalityParser.RPAREN);

	            }
	            break;

	        case 18:
	            localctx = new EventuallyMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 109;
	            this.match(ModalityParser.EVENTUALLY);
	            this.state = 110;
	            this.match(ModalityParser.LPAREN);
	            this.state = 111;
	            localctx.inner_formula = this.formula(0);
	            this.state = 112;
	            this.match(ModalityParser.RPAREN);
	            this.state = 118;
	            this._errHandler.sync(this);
	            var la_ = this._interp.adaptivePredict(this._input,4,this._ctx);
	            if(la_===1) {
	                this.state = 113;
	                this.match(ModalityParser.UNTIL);
	                this.state = 114;
	                this.match(ModalityParser.LPAREN);
	                this.state = 115;
	                localctx.until_formula = this.formula(0);
	                this.state = 116;
	                this.match(ModalityParser.RPAREN);

	            }
	            break;

	        case 19:
	            localctx = new UntilMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 120;
	            this.match(ModalityParser.UNTIL);
	            this.state = 121;
	            this.match(ModalityParser.LPAREN);
	            this.state = 122;
	            localctx.pre_formula = this.formula(0);
	            this.state = 123;
	            this.match(ModalityParser.COMMA);
	            this.state = 124;
	            localctx.post_formula = this.formula(0);
	            this.state = 125;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 20:
	            localctx = new WhenAlsoFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 127;
	            this.match(ModalityParser.WHEN);
	            this.state = 128;
	            localctx.when_formula = this.formula(0);
	            this.state = 129;
	            this.match(ModalityParser.ALSO);
	            this.state = 130;
	            localctx.also_formula = this.formula(0);
	            break;

	        case 21:
	            localctx = new WhenNextFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 132;
	            this.match(ModalityParser.WHEN);
	            this.state = 133;
	            localctx.when_formula = this.formula(0);
	            this.state = 134;
	            this.match(ModalityParser.NEXT);
	            this.state = 135;
	            localctx.next_formula = this.formula(0);
	            break;

	        case 22:
	            localctx = new ParenFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 137;
	            this.match(ModalityParser.LPAREN);
	            this.state = 138;
	            localctx.inner = this.formula(0);
	            this.state = 139;
	            this.match(ModalityParser.RPAREN);
	            break;

	        }
	        this._ctx.stop = this._input.LT(-1);
	        this.state = 155;
	        this._errHandler.sync(this);
	        var _alt = this._interp.adaptivePredict(this._input,7,this._ctx)
	        while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	            if(_alt===1) {
	                if(this._parseListeners!==null) {
	                    this.triggerExitRuleEvent();
	                }
	                _prevctx = localctx;
	                this.state = 153;
	                this._errHandler.sync(this);
	                var la_ = this._interp.adaptivePredict(this._input,6,this._ctx);
	                switch(la_) {
	                case 1:
	                    localctx = new OrFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 143;
	                    if (!( this.precpred(this._ctx, 23))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 23)");
	                    }
	                    this.state = 144;
	                    this.match(ModalityParser.OR);
	                    this.state = 145;
	                    localctx.right = this.formula(24);
	                    break;

	                case 2:
	                    localctx = new AndFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 146;
	                    if (!( this.precpred(this._ctx, 22))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 22)");
	                    }
	                    this.state = 147;
	                    this.match(ModalityParser.AND);
	                    this.state = 148;
	                    localctx.right = this.formula(23);
	                    break;

	                case 3:
	                    localctx = new ImpliesFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 149;
	                    if (!( this.precpred(this._ctx, 21))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 21)");
	                    }
	                    this.state = 150;
	                    this.implication();
	                    this.state = 151;
	                    localctx.right = this.formula(22);
	                    break;

	                } 
	            }
	            this.state = 157;
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
	        this.state = 158;
	        localctx.name = this.match(ModalityParser.NAME);
	        this.state = 159;
	        this.match(ModalityParser.LPAREN);
	        this.state = 168;
	        this._errHandler.sync(this);
	        _la = this._input.LA(1);
	        if(((((_la - 6)) & ~0x1f) === 0 && ((1 << (_la - 6)) & 369098755) !== 0)) {
	            this.state = 160;
	            this.arg();
	            this.state = 165;
	            this._errHandler.sync(this);
	            _la = this._input.LA(1);
	            while(_la===22) {
	                this.state = 161;
	                this.match(ModalityParser.COMMA);
	                this.state = 162;
	                this.arg();
	                this.state = 167;
	                this._errHandler.sync(this);
	                _la = this._input.LA(1);
	            }
	        }

	        this.state = 170;
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
	        this.state = 172;
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
	        this.state = 174;
	        localctx.theSign = this.sign();
	        this.state = 178;
	        this._errHandler.sync(this);
	        _la = this._input.LA(1);
	        while(_la===36) {
	            this.state = 175;
	            this.match(ModalityParser.WS);
	            this.state = 180;
	            this._errHandler.sync(this);
	            _la = this._input.LA(1);
	        }
	        this.state = 181;
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
	        this.state = 183;
	        _la = this._input.LA(1);
	        if(!((((_la) & ~0x1f) === 0 && ((1 << _la) & 58720256) !== 0))) {
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
	        this.state = 189;
	        this._errHandler.sync(this);
	        var la_ = this._interp.adaptivePredict(this._input,11,this._ctx);
	        switch(la_) {
	        case 1:
	            this.enterOuterAlt(localctx, 1);
	            this.state = 185;
	            this.match(ModalityParser.TRUE);
	            break;

	        case 2:
	            this.enterOuterAlt(localctx, 2);
	            this.state = 186;
	            this.match(ModalityParser.FALSE);
	            break;

	        case 3:
	            this.enterOuterAlt(localctx, 3);
	            this.state = 187;
	            this.match(ModalityParser.NAME);
	            break;

	        case 4:
	            this.enterOuterAlt(localctx, 4);
	            this.state = 188;
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



	implication() {
	    let localctx = new ImplicationContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 14, ModalityParser.RULE_implication);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 191;
	        _la = this._input.LA(1);
	        if(!((((_la) & ~0x1f) === 0 && ((1 << _la) & 201328640) !== 0))) {
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



	notOp() {
	    let localctx = new NotOpContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 16, ModalityParser.RULE_notOp);
	    var _la = 0;
	    try {
	        this.enterOuterAlt(localctx, 1);
	        this.state = 193;
	        _la = this._input.LA(1);
	        if(!(_la===10 || _la===35)) {
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



	arg() {
	    let localctx = new ArgContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 18, ModalityParser.RULE_arg);
	    try {
	        this.state = 200;
	        this._errHandler.sync(this);
	        switch(this._input.LA(1)) {
	        case 6:
	            localctx = new TrueArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 1);
	            this.state = 195;
	            this.match(ModalityParser.TRUE);
	            break;
	        case 7:
	            localctx = new FalseArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 2);
	            this.state = 196;
	            this.match(ModalityParser.FALSE);
	            break;
	        case 31:
	            localctx = new StringArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 3);
	            this.state = 197;
	            this.match(ModalityParser.STRING);
	            break;
	        case 32:
	            localctx = new NumberArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 4);
	            this.state = 198;
	            this.match(ModalityParser.NUMBER);
	            break;
	        case 34:
	            localctx = new VariableArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 5);
	            this.state = 199;
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
ModalityParser.IMPLIES = 11;
ModalityParser.WHEN = 12;
ModalityParser.ALSO = 13;
ModalityParser.NEXT = 14;
ModalityParser.LBOX = 15;
ModalityParser.RBOX = 16;
ModalityParser.LDIA = 17;
ModalityParser.RDIA = 18;
ModalityParser.LPAREN = 19;
ModalityParser.RPAREN = 20;
ModalityParser.STAR = 21;
ModalityParser.COMMA = 22;
ModalityParser.PLUS = 23;
ModalityParser.MINUS = 24;
ModalityParser.QMARK = 25;
ModalityParser.ARROW = 26;
ModalityParser.FAT_ARROW = 27;
ModalityParser.LFP = 28;
ModalityParser.GFP = 29;
ModalityParser.NAME = 30;
ModalityParser.STRING = 31;
ModalityParser.NUMBER = 32;
ModalityParser.STATE_SET_VARIABLE = 33;
ModalityParser.VARIABLE = 34;
ModalityParser.BANG = 35;
ModalityParser.WS = 36;
ModalityParser.LINE_COMMENT = 37;

ModalityParser.RULE_expression = 0;
ModalityParser.RULE_formula = 1;
ModalityParser.RULE_functionProp = 2;
ModalityParser.RULE_unsignedProp = 3;
ModalityParser.RULE_signedProp = 4;
ModalityParser.RULE_sign = 5;
ModalityParser.RULE_prop = 6;
ModalityParser.RULE_implication = 7;
ModalityParser.RULE_notOp = 8;
ModalityParser.RULE_arg = 9;

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

class NotFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        super.copyFrom(ctx);
    }

	notOp() {
	    return this.getTypedRuleContext(NotOpContext,0);
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

class BoxGuardImpliesFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        this.right = null;;
        super.copyFrom(ctx);
    }

	LBOX() {
	    return this.getToken(ModalityParser.LBOX, 0);
	};

	RBOX() {
	    return this.getToken(ModalityParser.RBOX, 0);
	};

	implication() {
	    return this.getTypedRuleContext(ImplicationContext,0);
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
	        listener.enterBoxGuardImpliesFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitBoxGuardImpliesFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitBoxGuardImpliesFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.BoxGuardImpliesFormulaContext = BoxGuardImpliesFormulaContext;

class DiamondGuardImpliesFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.inner = null;;
        this.right = null;;
        super.copyFrom(ctx);
    }

	LDIA() {
	    return this.getToken(ModalityParser.LDIA, 0);
	};

	RDIA() {
	    return this.getToken(ModalityParser.RDIA, 0);
	};

	implication() {
	    return this.getTypedRuleContext(ImplicationContext,0);
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
	        listener.enterDiamondGuardImpliesFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitDiamondGuardImpliesFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitDiamondGuardImpliesFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.DiamondGuardImpliesFormulaContext = DiamondGuardImpliesFormulaContext;

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

class UntilMacroContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.pre_formula = null;;
        this.post_formula = null;;
        super.copyFrom(ctx);
    }

	UNTIL() {
	    return this.getToken(ModalityParser.UNTIL, 0);
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
	        listener.enterUntilMacro(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitUntilMacro(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitUntilMacro(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.UntilMacroContext = UntilMacroContext;

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

class ImpliesFormulaContext extends FormulaContext {

    constructor(parser, ctx) {
        super(parser);
        this.left = null;;
        this.right = null;;
        super.copyFrom(ctx);
    }

	implication() {
	    return this.getTypedRuleContext(ImplicationContext,0);
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
	        listener.enterImpliesFormula(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitImpliesFormula(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitImpliesFormula(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}

ModalityParser.ImpliesFormulaContext = ImpliesFormulaContext;

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



class ImplicationContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalityParser.RULE_implication;
    }

	IMPLIES() {
	    return this.getToken(ModalityParser.IMPLIES, 0);
	};

	ARROW() {
	    return this.getToken(ModalityParser.ARROW, 0);
	};

	FAT_ARROW() {
	    return this.getToken(ModalityParser.FAT_ARROW, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterImplication(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitImplication(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitImplication(this);
	    } else {
	        return visitor.visitChildren(this);
	    }
	}


}



class NotOpContext extends antlr4.ParserRuleContext {

    constructor(parser, parent, invokingState) {
        if(parent===undefined) {
            parent = null;
        }
        if(invokingState===undefined || invokingState===null) {
            invokingState = -1;
        }
        super(parent, invokingState);
        this.parser = parser;
        this.ruleIndex = ModalityParser.RULE_notOp;
    }

	NOT() {
	    return this.getToken(ModalityParser.NOT, 0);
	};

	BANG() {
	    return this.getToken(ModalityParser.BANG, 0);
	};

	enterRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.enterNotOp(this);
		}
	}

	exitRule(listener) {
	    if(listener instanceof ModalityListener ) {
	        listener.exitNotOp(this);
		}
	}

	accept(visitor) {
	    if ( visitor instanceof ModalityVisitor ) {
	        return visitor.visitNotOp(this);
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


ModalityParser.ExpressionContext = ExpressionContext; 
ModalityParser.FormulaContext = FormulaContext; 
ModalityParser.FunctionPropContext = FunctionPropContext; 
ModalityParser.UnsignedPropContext = UnsignedPropContext; 
ModalityParser.SignedPropContext = SignedPropContext; 
ModalityParser.SignContext = SignContext; 
ModalityParser.PropContext = PropContext; 
ModalityParser.ImplicationContext = ImplicationContext; 
ModalityParser.NotOpContext = NotOpContext; 
ModalityParser.ArgContext = ArgContext; 
