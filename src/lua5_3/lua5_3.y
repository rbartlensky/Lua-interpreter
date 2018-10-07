%start block
%%
block
    : statlistopt retstatopt;
statlist
    : statlist stat
    | stat
    ;
statlistopt
    : statlist
    |
    ;
prefixexp
    : var
    | functioncall
    | "LBRACKET" exp "RBRACKET"
    ;
stat
    : "SEMICOL"
    | varlist "EQ" explist
    | functioncall
    | label
    | "BREAK"
    | "GOTO" "NAME"
    | "DO" block "END"
    | "WHILE" exp "DO" block "END"
    | "REPEAT" block "UNTIL" exp
    | "IF" exp "THEN" block elselistopt elseopt "END"
    | "FOR" "NAME" "EQ" exp "COMMA" explist "DO" block "END"
    | "FOR" namelist "IN" explist "DO" block "END"
    | "FUNCTION" funcname funcbody
    | "LOCAL" "FUNCTION" "NAME" funcbody
    | "LOCAL" namelist eqexplistopt
    ;
retstatopt
    : "RETURN" explistopt semicolonopt
    |
    ;
label
    : "COLCOL" "NAME" "COLCOL"
    ;
funcname
    : "NAME" funcnamelist
    | "NAME" funcnamelist "COL" "NAME"
    ;
funcnamelist
    : funcnamelist "DOT" "NAME"
    |
    ;
varlist
    : varlist "COMMA" var
    | var
    ;
var : "NAME"
    | prefixexp "LSQUARE" exp "RSQUARE"
    | prefixexp "DOT" "NAME"
    ;
explist
    : explist "COMMA" exp
    | exp
    ;
explistopt
    : explist
    |
    ;
eqexplistopt
    : "EQ" explist
    |
    ;
elselistopt
    : elselist
    |
    ;
elselist
    : elselist "ELSEIF" exp "THEN" block
    | "ELSEIF" exp "THEN" block
    ;
elseopt
    : "ELSE" block
    |
    ;
semicolonopt
    : "SEMICOL"
    |
    ;
namelist
    : namelist "COMMA" "NAME"
    | "NAME"
    ;
// Lua has 12 precedence levels which we encode as exp[0-11] (where "exp0" is,
// for convenience, simply called "exp").
exp : exp "OR" exp1
    | exp1
    ;
exp1: exp1 "AND" exp2
    | exp2
    ;
exp2: exp2 "LT" exp3
    | exp2 "GT" exp3
    | exp2 "LE" exp3
    | exp2 "GE" exp3
    | exp2 "NOTEQ" exp3
    | exp2 "EQEQ" exp3
    | exp3
    ;
exp3: exp3 "PIPE" exp4
    | exp4
    ;
exp4: exp4 "TILDE" exp5
    | exp5
    ;
exp5: exp5 "AMP" exp6
    | exp6
    ;
exp6: exp6 "LTLT" exp7
    | exp6 "GTGT" exp7
    | exp7
    ;
exp7: exp8 "DOTDOT" exp7
    | exp8
    ;
exp8: exp8 "PLUS" exp9
    | exp8 "MINUS" exp9
    | exp9
    ;
exp9: exp9 "STAR" exp10
    | exp9 "FSLASH" exp10
    | exp9 "FSFS" exp10
    | exp9 "MOD" exp10
    | exp10
    ;
exp10
    : "NOT" exp10
    | "HASH" exp10
    | "MINUS" exp10
    | "TILDE" exp10
    | exp11
    ;
exp11
    : exp12 "CARET" exp10
    | exp12
    ;
exp12
    : "NIL"
    | "FALSE"
    | "TRUE"
    | "NUMERAL"
    | literalstring
    | "DOTDOTDOT"
    | functiondef
    | prefixexp
    | tableconstructor
    ;
functioncall
    : prefixexp args
    | prefixexp "COL" "NAME" args
    ;
args: "LBRACKET" explistopt "RBRACKET"
    | tableconstructor
    | literalstring
    ;
functiondef
    : "FUNCTION" funcbody
    ;
funcbody
    : "LBRACKET" parlist "RBRACKET" block "END";
parlist
    : namelist "COMMA" "DOTDOTDOT"
    | namelist
    | "DOTDOTDOT"
    |
    ;
tableconstructor
    : "LCURLY" fieldlistopt "RCURLY";
fieldlistopt
    : fieldlist fieldsepopt
    |
    ;
fieldlist
    : fieldlist fieldsep field
    | field
    ;
field
    : "LSQUARE" exp "RSQUARE" "EQ" exp
    | "NAME" "EQ" exp
    | exp
    ;
fieldsep
    : "COMMA"
    | "SEMICOL"
    ;
fieldsepopt
    : fieldsep
    |
    ;
literalstring
    : "SHORT_STR"
    | "LONG_STR"
    ;
