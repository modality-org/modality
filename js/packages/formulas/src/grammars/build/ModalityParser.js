// jshint ignore: start
import antlr4 from 'antlr4';
import ModalityListener from './ModalityListener.js';
import ModalityVisitor from './ModalityVisitor.js';

const serializedATN = [4,1,36,198,2,0,7,0,2,1,7,1,2,2,7,2,2,3,7,3,2,4,7,
4,2,5,7,5,2,6,7,6,2,7,7,7,2,8,7,8,1,0,1,0,1,0,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,5,1,31,8,1,10,1,12,1,34,9,1,1,1,4,1,37,8,1,11,1,12,1,38,3,1,41,
8,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,1,105,8,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,3,1,116,8,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,1,139,8,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,5,1,151,8,1,10,1,12,1,154,9,1,1,2,1,2,1,2,1,2,1,2,5,2,161,
8,2,10,2,12,2,164,9,2,3,2,166,8,2,1,2,1,2,1,3,1,3,1,4,1,4,5,4,174,8,4,10,
4,12,4,177,9,4,1,4,1,4,1,5,1,5,1,6,1,6,1,6,1,6,3,6,187,8,6,1,7,1,7,1,8,1,
8,1,8,1,8,1,8,3,8,196,8,8,1,8,0,1,2,9,0,2,4,6,8,10,12,14,16,0,2,1,0,23,25,
2,0,11,11,26,27,227,0,18,1,0,0,0,2,138,1,0,0,0,4,155,1,0,0,0,6,169,1,0,0,
0,8,171,1,0,0,0,10,180,1,0,0,0,12,186,1,0,0,0,14,188,1,0,0,0,16,195,1,0,
0,0,18,19,3,2,1,0,19,20,5,0,0,1,20,1,1,0,0,0,21,22,6,1,-1,0,22,139,5,6,0,
0,23,139,5,7,0,0,24,25,5,24,0,0,25,139,3,2,1,20,26,27,5,10,0,0,27,139,3,
2,1,19,28,32,3,6,3,0,29,31,3,8,4,0,30,29,1,0,0,0,31,34,1,0,0,0,32,30,1,0,
0,0,32,33,1,0,0,0,33,41,1,0,0,0,34,32,1,0,0,0,35,37,3,8,4,0,36,35,1,0,0,
0,37,38,1,0,0,0,38,36,1,0,0,0,38,39,1,0,0,0,39,41,1,0,0,0,40,28,1,0,0,0,
40,36,1,0,0,0,41,139,1,0,0,0,42,139,5,33,0,0,43,44,5,15,0,0,44,45,5,16,0,
0,45,139,3,2,1,16,46,47,5,17,0,0,47,48,5,18,0,0,48,139,3,2,1,15,49,50,5,
15,0,0,50,51,3,2,1,0,51,52,5,16,0,0,52,53,3,14,7,0,53,54,3,2,1,0,54,139,
1,0,0,0,55,56,5,17,0,0,56,57,3,2,1,0,57,58,5,18,0,0,58,59,3,14,7,0,59,60,
3,2,1,0,60,139,1,0,0,0,61,62,5,15,0,0,62,63,3,2,1,0,63,64,5,16,0,0,64,65,
3,2,1,0,65,139,1,0,0,0,66,67,5,17,0,0,67,68,3,2,1,0,68,69,5,18,0,0,69,70,
3,2,1,0,70,139,1,0,0,0,71,72,5,28,0,0,72,73,5,19,0,0,73,74,3,2,1,0,74,75,
5,22,0,0,75,76,3,2,1,0,76,77,5,20,0,0,77,139,1,0,0,0,78,79,5,29,0,0,79,80,
5,19,0,0,80,81,3,2,1,0,81,82,5,22,0,0,82,83,3,2,1,0,83,84,5,20,0,0,84,139,
1,0,0,0,85,86,5,1,0,0,86,87,5,19,0,0,87,88,3,2,1,0,88,89,5,20,0,0,89,139,
1,0,0,0,90,91,5,2,0,0,91,92,5,19,0,0,92,93,3,2,1,0,93,94,5,20,0,0,94,139,
1,0,0,0,95,96,5,3,0,0,96,97,5,19,0,0,97,98,3,2,1,0,98,104,5,20,0,0,99,100,
5,5,0,0,100,101,5,19,0,0,101,102,3,2,1,0,102,103,5,20,0,0,103,105,1,0,0,
0,104,99,1,0,0,0,104,105,1,0,0,0,105,139,1,0,0,0,106,107,5,4,0,0,107,108,
5,19,0,0,108,109,3,2,1,0,109,115,5,20,0,0,110,111,5,5,0,0,111,112,5,19,0,
0,112,113,3,2,1,0,113,114,5,20,0,0,114,116,1,0,0,0,115,110,1,0,0,0,115,116,
1,0,0,0,116,139,1,0,0,0,117,118,5,5,0,0,118,119,5,19,0,0,119,120,3,2,1,0,
120,121,5,22,0,0,121,122,3,2,1,0,122,123,5,20,0,0,123,139,1,0,0,0,124,125,
5,12,0,0,125,126,3,2,1,0,126,127,5,13,0,0,127,128,3,2,1,0,128,139,1,0,0,
0,129,130,5,12,0,0,130,131,3,2,1,0,131,132,5,14,0,0,132,133,3,2,1,0,133,
139,1,0,0,0,134,135,5,19,0,0,135,136,3,2,1,0,136,137,5,20,0,0,137,139,1,
0,0,0,138,21,1,0,0,0,138,23,1,0,0,0,138,24,1,0,0,0,138,26,1,0,0,0,138,40,
1,0,0,0,138,42,1,0,0,0,138,43,1,0,0,0,138,46,1,0,0,0,138,49,1,0,0,0,138,
55,1,0,0,0,138,61,1,0,0,0,138,66,1,0,0,0,138,71,1,0,0,0,138,78,1,0,0,0,138,
85,1,0,0,0,138,90,1,0,0,0,138,95,1,0,0,0,138,106,1,0,0,0,138,117,1,0,0,0,
138,124,1,0,0,0,138,129,1,0,0,0,138,134,1,0,0,0,139,152,1,0,0,0,140,141,
10,23,0,0,141,142,5,9,0,0,142,151,3,2,1,24,143,144,10,22,0,0,144,145,5,8,
0,0,145,151,3,2,1,23,146,147,10,21,0,0,147,148,3,14,7,0,148,149,3,2,1,22,
149,151,1,0,0,0,150,140,1,0,0,0,150,143,1,0,0,0,150,146,1,0,0,0,151,154,
1,0,0,0,152,150,1,0,0,0,152,153,1,0,0,0,153,3,1,0,0,0,154,152,1,0,0,0,155,
156,5,30,0,0,156,165,5,19,0,0,157,162,3,16,8,0,158,159,5,22,0,0,159,161,
3,16,8,0,160,158,1,0,0,0,161,164,1,0,0,0,162,160,1,0,0,0,162,163,1,0,0,0,
163,166,1,0,0,0,164,162,1,0,0,0,165,157,1,0,0,0,165,166,1,0,0,0,166,167,
1,0,0,0,167,168,5,20,0,0,168,5,1,0,0,0,169,170,3,12,6,0,170,7,1,0,0,0,171,
175,3,10,5,0,172,174,5,35,0,0,173,172,1,0,0,0,174,177,1,0,0,0,175,173,1,
0,0,0,175,176,1,0,0,0,176,178,1,0,0,0,177,175,1,0,0,0,178,179,3,12,6,0,179,
9,1,0,0,0,180,181,7,0,0,0,181,11,1,0,0,0,182,187,5,6,0,0,183,187,5,7,0,0,
184,187,5,30,0,0,185,187,3,4,2,0,186,182,1,0,0,0,186,183,1,0,0,0,186,184,
1,0,0,0,186,185,1,0,0,0,187,13,1,0,0,0,188,189,7,1,0,0,189,15,1,0,0,0,190,
196,5,6,0,0,191,196,5,7,0,0,192,196,5,31,0,0,193,196,5,32,0,0,194,196,5,
34,0,0,195,190,1,0,0,0,195,191,1,0,0,0,195,192,1,0,0,0,195,193,1,0,0,0,195,
194,1,0,0,0,196,17,1,0,0,0,13,32,38,40,104,115,138,150,152,162,165,175,186,
195];


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
                            "'lfp'", "'gfp'" ];
    static symbolicNames = [ null, "MUST", "CAN", "ALWAYS", "EVENTUALLY", 
                             "UNTIL", "TRUE", "FALSE", "AND", "OR", "NOT", 
                             "IMPLIES", "WHEN", "ALSO", "NEXT", "LBOX", 
                             "RBOX", "LDIA", "RDIA", "LPAREN", "RPAREN", 
                             "STAR", "COMMA", "PLUS", "MINUS", "QMARK", 
                             "ARROW", "FAT_ARROW", "LFP", "GFP", "NAME", 
                             "STRING", "NUMBER", "STATE_SET_VARIABLE", "VARIABLE", 
                             "WS", "LINE_COMMENT" ];
    static ruleNames = [ "expression", "formula", "functionProp", "unsignedProp", 
                         "signedProp", "sign", "prop", "implication", "arg" ];

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
	        this.state = 18;
	        localctx.f = this.formula(0);
	        this.state = 19;
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
	        this.state = 138;
	        this._errHandler.sync(this);
	        var la_ = this._interp.adaptivePredict(this._input,5,this._ctx);
	        switch(la_) {
	        case 1:
	            localctx = new TrueAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;

	            this.state = 22;
	            this.match(ModalityParser.TRUE);
	            break;

	        case 2:
	            localctx = new FalseAtomContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 23;
	            this.match(ModalityParser.FALSE);
	            break;

	        case 3:
	            localctx = new NegatedFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 24;
	            this.match(ModalityParser.MINUS);
	            this.state = 25;
	            localctx.inner = this.formula(20);
	            break;

	        case 4:
	            localctx = new NotFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 26;
	            this.match(ModalityParser.NOT);
	            this.state = 27;
	            localctx.inner = this.formula(19);
	            break;

	        case 5:
	            localctx = new PropsSetContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 40;
	            this._errHandler.sync(this);
	            switch(this._input.LA(1)) {
	            case 6:
	            case 7:
	            case 30:
	                this.state = 28;
	                this.unsignedProp();
	                this.state = 32;
	                this._errHandler.sync(this);
	                var _alt = this._interp.adaptivePredict(this._input,0,this._ctx)
	                while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	                    if(_alt===1) {
	                        this.state = 29;
	                        this.signedProp(); 
	                    }
	                    this.state = 34;
	                    this._errHandler.sync(this);
	                    _alt = this._interp.adaptivePredict(this._input,0,this._ctx);
	                }

	                break;
	            case 23:
	            case 24:
	            case 25:
	                this.state = 36; 
	                this._errHandler.sync(this);
	                var _alt = 1;
	                do {
	                	switch (_alt) {
	                	case 1:
	                		this.state = 35;
	                		this.signedProp();
	                		break;
	                	default:
	                		throw new antlr4.error.NoViableAltException(this);
	                	}
	                	this.state = 38; 
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
	            this.state = 42;
	            this.match(ModalityParser.STATE_SET_VARIABLE);
	            break;

	        case 7:
	            localctx = new EmptyBoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 43;
	            this.match(ModalityParser.LBOX);
	            this.state = 44;
	            this.match(ModalityParser.RBOX);
	            this.state = 45;
	            localctx.outer = this.formula(16);
	            break;

	        case 8:
	            localctx = new EmptyDiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 46;
	            this.match(ModalityParser.LDIA);
	            this.state = 47;
	            this.match(ModalityParser.RDIA);
	            this.state = 48;
	            localctx.outer = this.formula(15);
	            break;

	        case 9:
	            localctx = new BoxGuardImpliesFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 49;
	            this.match(ModalityParser.LBOX);
	            this.state = 50;
	            localctx.inner = this.formula(0);
	            this.state = 51;
	            this.match(ModalityParser.RBOX);
	            this.state = 52;
	            this.implication();
	            this.state = 53;
	            localctx.right = this.formula(0);
	            break;

	        case 10:
	            localctx = new DiamondGuardImpliesFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 55;
	            this.match(ModalityParser.LDIA);
	            this.state = 56;
	            localctx.inner = this.formula(0);
	            this.state = 57;
	            this.match(ModalityParser.RDIA);
	            this.state = 58;
	            this.implication();
	            this.state = 59;
	            localctx.right = this.formula(0);
	            break;

	        case 11:
	            localctx = new BoxFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 61;
	            this.match(ModalityParser.LBOX);
	            this.state = 62;
	            localctx.inner = this.formula(0);
	            this.state = 63;
	            this.match(ModalityParser.RBOX);
	            this.state = 64;
	            localctx.outer = this.formula(0);
	            break;

	        case 12:
	            localctx = new DiamondFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 66;
	            this.match(ModalityParser.LDIA);
	            this.state = 67;
	            localctx.inner = this.formula(0);
	            this.state = 68;
	            this.match(ModalityParser.RDIA);
	            this.state = 69;
	            localctx.outer = this.formula(0);
	            break;

	        case 13:
	            localctx = new LfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 71;
	            this.match(ModalityParser.LFP);
	            this.state = 72;
	            this.match(ModalityParser.LPAREN);
	            this.state = 73;
	            localctx.stateSetVariable = this.formula(0);
	            this.state = 74;
	            this.match(ModalityParser.COMMA);
	            this.state = 75;
	            localctx.inner = this.formula(0);
	            this.state = 76;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 14:
	            localctx = new GfpFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 78;
	            this.match(ModalityParser.GFP);
	            this.state = 79;
	            this.match(ModalityParser.LPAREN);
	            this.state = 80;
	            localctx.stateSetVariable = this.formula(0);
	            this.state = 81;
	            this.match(ModalityParser.COMMA);
	            this.state = 82;
	            localctx.inner = this.formula(0);
	            this.state = 83;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 15:
	            localctx = new MustMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 85;
	            this.match(ModalityParser.MUST);
	            this.state = 86;
	            this.match(ModalityParser.LPAREN);
	            this.state = 87;
	            this.formula(0);
	            this.state = 88;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 16:
	            localctx = new CanMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 90;
	            this.match(ModalityParser.CAN);
	            this.state = 91;
	            this.match(ModalityParser.LPAREN);
	            this.state = 92;
	            this.formula(0);
	            this.state = 93;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 17:
	            localctx = new AlwaysMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 95;
	            this.match(ModalityParser.ALWAYS);
	            this.state = 96;
	            this.match(ModalityParser.LPAREN);
	            this.state = 97;
	            localctx.inner_formula = this.formula(0);
	            this.state = 98;
	            this.match(ModalityParser.RPAREN);
	            this.state = 104;
	            this._errHandler.sync(this);
	            var la_ = this._interp.adaptivePredict(this._input,3,this._ctx);
	            if(la_===1) {
	                this.state = 99;
	                this.match(ModalityParser.UNTIL);
	                this.state = 100;
	                this.match(ModalityParser.LPAREN);
	                this.state = 101;
	                localctx.until_formula = this.formula(0);
	                this.state = 102;
	                this.match(ModalityParser.RPAREN);

	            }
	            break;

	        case 18:
	            localctx = new EventuallyMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 106;
	            this.match(ModalityParser.EVENTUALLY);
	            this.state = 107;
	            this.match(ModalityParser.LPAREN);
	            this.state = 108;
	            localctx.inner_formula = this.formula(0);
	            this.state = 109;
	            this.match(ModalityParser.RPAREN);
	            this.state = 115;
	            this._errHandler.sync(this);
	            var la_ = this._interp.adaptivePredict(this._input,4,this._ctx);
	            if(la_===1) {
	                this.state = 110;
	                this.match(ModalityParser.UNTIL);
	                this.state = 111;
	                this.match(ModalityParser.LPAREN);
	                this.state = 112;
	                localctx.until_formula = this.formula(0);
	                this.state = 113;
	                this.match(ModalityParser.RPAREN);

	            }
	            break;

	        case 19:
	            localctx = new UntilMacroContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 117;
	            this.match(ModalityParser.UNTIL);
	            this.state = 118;
	            this.match(ModalityParser.LPAREN);
	            this.state = 119;
	            localctx.pre_formula = this.formula(0);
	            this.state = 120;
	            this.match(ModalityParser.COMMA);
	            this.state = 121;
	            localctx.post_formula = this.formula(0);
	            this.state = 122;
	            this.match(ModalityParser.RPAREN);
	            break;

	        case 20:
	            localctx = new WhenAlsoFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 124;
	            this.match(ModalityParser.WHEN);
	            this.state = 125;
	            localctx.when_formula = this.formula(0);
	            this.state = 126;
	            this.match(ModalityParser.ALSO);
	            this.state = 127;
	            localctx.also_formula = this.formula(0);
	            break;

	        case 21:
	            localctx = new WhenNextFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 129;
	            this.match(ModalityParser.WHEN);
	            this.state = 130;
	            localctx.when_formula = this.formula(0);
	            this.state = 131;
	            this.match(ModalityParser.NEXT);
	            this.state = 132;
	            localctx.next_formula = this.formula(0);
	            break;

	        case 22:
	            localctx = new ParenFormulaContext(this, localctx);
	            this._ctx = localctx;
	            _prevctx = localctx;
	            this.state = 134;
	            this.match(ModalityParser.LPAREN);
	            this.state = 135;
	            localctx.inner = this.formula(0);
	            this.state = 136;
	            this.match(ModalityParser.RPAREN);
	            break;

	        }
	        this._ctx.stop = this._input.LT(-1);
	        this.state = 152;
	        this._errHandler.sync(this);
	        var _alt = this._interp.adaptivePredict(this._input,7,this._ctx)
	        while(_alt!=2 && _alt!=antlr4.atn.ATN.INVALID_ALT_NUMBER) {
	            if(_alt===1) {
	                if(this._parseListeners!==null) {
	                    this.triggerExitRuleEvent();
	                }
	                _prevctx = localctx;
	                this.state = 150;
	                this._errHandler.sync(this);
	                var la_ = this._interp.adaptivePredict(this._input,6,this._ctx);
	                switch(la_) {
	                case 1:
	                    localctx = new OrFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 140;
	                    if (!( this.precpred(this._ctx, 23))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 23)");
	                    }
	                    this.state = 141;
	                    this.match(ModalityParser.OR);
	                    this.state = 142;
	                    localctx.right = this.formula(24);
	                    break;

	                case 2:
	                    localctx = new AndFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 143;
	                    if (!( this.precpred(this._ctx, 22))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 22)");
	                    }
	                    this.state = 144;
	                    this.match(ModalityParser.AND);
	                    this.state = 145;
	                    localctx.right = this.formula(23);
	                    break;

	                case 3:
	                    localctx = new ImpliesFormulaContext(this, new FormulaContext(this, _parentctx, _parentState));
	                    localctx.left = _prevctx;
	                    this.pushNewRecursionContext(localctx, _startState, ModalityParser.RULE_formula);
	                    this.state = 146;
	                    if (!( this.precpred(this._ctx, 21))) {
	                        throw new antlr4.error.FailedPredicateException(this, "this.precpred(this._ctx, 21)");
	                    }
	                    this.state = 147;
	                    this.implication();
	                    this.state = 148;
	                    localctx.right = this.formula(22);
	                    break;

	                } 
	            }
	            this.state = 154;
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
	        this.state = 155;
	        localctx.name = this.match(ModalityParser.NAME);
	        this.state = 156;
	        this.match(ModalityParser.LPAREN);
	        this.state = 165;
	        this._errHandler.sync(this);
	        _la = this._input.LA(1);
	        if(((((_la - 6)) & ~0x1f) === 0 && ((1 << (_la - 6)) & 369098755) !== 0)) {
	            this.state = 157;
	            this.arg();
	            this.state = 162;
	            this._errHandler.sync(this);
	            _la = this._input.LA(1);
	            while(_la===22) {
	                this.state = 158;
	                this.match(ModalityParser.COMMA);
	                this.state = 159;
	                this.arg();
	                this.state = 164;
	                this._errHandler.sync(this);
	                _la = this._input.LA(1);
	            }
	        }

	        this.state = 167;
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
	        this.state = 169;
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
	        this.state = 171;
	        localctx.theSign = this.sign();
	        this.state = 175;
	        this._errHandler.sync(this);
	        _la = this._input.LA(1);
	        while(_la===35) {
	            this.state = 172;
	            this.match(ModalityParser.WS);
	            this.state = 177;
	            this._errHandler.sync(this);
	            _la = this._input.LA(1);
	        }
	        this.state = 178;
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
	        this.state = 180;
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
	        this.state = 186;
	        this._errHandler.sync(this);
	        var la_ = this._interp.adaptivePredict(this._input,11,this._ctx);
	        switch(la_) {
	        case 1:
	            this.enterOuterAlt(localctx, 1);
	            this.state = 182;
	            this.match(ModalityParser.TRUE);
	            break;

	        case 2:
	            this.enterOuterAlt(localctx, 2);
	            this.state = 183;
	            this.match(ModalityParser.FALSE);
	            break;

	        case 3:
	            this.enterOuterAlt(localctx, 3);
	            this.state = 184;
	            this.match(ModalityParser.NAME);
	            break;

	        case 4:
	            this.enterOuterAlt(localctx, 4);
	            this.state = 185;
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
	        this.state = 188;
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



	arg() {
	    let localctx = new ArgContext(this, this._ctx, this.state);
	    this.enterRule(localctx, 16, ModalityParser.RULE_arg);
	    try {
	        this.state = 195;
	        this._errHandler.sync(this);
	        switch(this._input.LA(1)) {
	        case 6:
	            localctx = new TrueArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 1);
	            this.state = 190;
	            this.match(ModalityParser.TRUE);
	            break;
	        case 7:
	            localctx = new FalseArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 2);
	            this.state = 191;
	            this.match(ModalityParser.FALSE);
	            break;
	        case 31:
	            localctx = new StringArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 3);
	            this.state = 192;
	            this.match(ModalityParser.STRING);
	            break;
	        case 32:
	            localctx = new NumberArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 4);
	            this.state = 193;
	            this.match(ModalityParser.NUMBER);
	            break;
	        case 34:
	            localctx = new VariableArgContext(this, localctx);
	            this.enterOuterAlt(localctx, 5);
	            this.state = 194;
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
ModalityParser.WS = 35;
ModalityParser.LINE_COMMENT = 36;

ModalityParser.RULE_expression = 0;
ModalityParser.RULE_formula = 1;
ModalityParser.RULE_functionProp = 2;
ModalityParser.RULE_unsignedProp = 3;
ModalityParser.RULE_signedProp = 4;
ModalityParser.RULE_sign = 5;
ModalityParser.RULE_prop = 6;
ModalityParser.RULE_implication = 7;
ModalityParser.RULE_arg = 8;

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
ModalityParser.ArgContext = ArgContext; 
