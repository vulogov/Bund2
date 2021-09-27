grammar Bund;

expressions
  : root_term*
;

root_term
   : ( ns
     | block
   )
;

ns
  : '[' name=(NAME|STRING) ':' (body+=term)* ';;'
  ;

block
  : '(' (body+=term)* ')'(':(' FUNCTOR=(SYS|SYSF|NAME) ')')?
  ;

term
  : ( ns
    | block
    | call_term
    | ref_call_term
    | boolean_term
    | integer_term
    | float_term
    | string_term
    | complex_term
    | datablock_term
    | matchblock_term
    | logicblock_term
    | mode_term
    | separate_term
    | function_term
    | generator_term
    | operator_term
    | lambda_term
    | generator_term
    | index_term
  );

  fterm
    : ( ns
      | block
      | call_term
      | ref_call_term
      | boolean_term
      | integer_term
      | float_term
      | string_term
      | complex_term
      | datablock_term
      | matchblock_term
      | logicblock_term
      | mode_term
      | separate_term
      | index_term
    );

data
  : ( boolean_term
    | integer_term
    | float_term
    | string_term
    | complex_term
    | call_term
    | separate_term
    | lambda_term
    | ref_call_term
  );

call_term
  : (PRE=NAME '@')? VALUE=(SYS|SYSF|CMD|NAME) (':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')?
  ;


ref_call_term:     '`' (PRE=NAME '@')? VALUE=(SYS|SYSF|CMD|NAME) (':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')? ;

boolean_term: (PRE=NAME '@')? VALUE=(TRUE|FALSE)    (':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')? ;
integer_term: (PRE=NAME '@')? VALUE=INTEGER         (':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')? ;
float_term:   (PRE=NAME '@')? VALUE=(FLOAT_NUMBER|'+Inf'|'NaN'|'-Inf'|'Inf')(':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')? ;
string_term:  (PRE=NAME '@')? VALUE=STRING          (':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')? ;
complex_term: (PRE=NAME '@')? VALUE=COMPLEX_NUMBER  (':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')? ;

mode_term:    VALUE=(TOBEGIN|TOEND) ;

separate_term: VALUE=SEPARATE ;

datablock_term
  : '(*' (body+=data)* ')'(':(' FUNCTOR=(SYS|SYSF|CMD|NAME) ')')?
  ;

matchblock_term
  : '(?' (body+=data)+ ')'
  ;

logicblock_term
  : HDR=('(true'|'(false') (body+=term)* ')'
  ;

function_term
  : '[' name=(SYS|SYSF|CMD|NAME) ']' (body+=fterm)* '.'
  ;

lambda_term
  : '[]' (body+=fterm)* '.'
  ;


operator_term
  : '[[' name=(SYS|SYSF|CMD|NAME) ']]' (body+=fterm)* '.'
  ;

generator_term
  : '[[[' name=(SYS|SYSF|CMD|NAME) ']]]' (body+=fterm)* '.'
  ;

index_term
  : '#' VALUE=INTEGER ('::' ENDVALUE=INTEGER)?
  ;


INTEGER
  :  (SIGN)? DECIMAL_INTEGER
  ;

DECIMAL_INTEGER
  : NON_ZERO_DIGIT DIGIT*
  | '0'+
  ;

FLOAT_NUMBER
  : EXPONENT_OR_POINT_FLOAT
  ;

STRING
  : SHORT_STRING
  | LONG_STRING
  ;

COMPLEX_NUMBER
  : '(' FLOAT_NUMBER SIGN FLOAT_NUMBER 'i)'
  ;


TRUE
  : 'true'
  | 'True'
  | 'TRUE'
  | 'T'
  | 'yes'
  | 'Yes'
  | 'YES'
  ;

FALSE
  : 'false'
  | 'False'
  | 'FALSE'
  | 'F'
  | 'no'
  | 'No'
  | 'NO'
  ;

SYS
  : ('_'|'%'|'?')+
  ;

CMD
  : ('→'|'↑'|'√'|'≉'|'≈'|'∐'|'∏'|'∇'|'∆'|'∪'|'∩'|'∉'|'∈'|'⊉'|'⊈'|'⊇'|'⊆'|'⊅'|'⊄'|'⊃'|'⊂'|'÷'|'\\'|'+'|'-'|'&'|'='|'<'|'>'|'*'|'×')+
  ;

SYSF
  : ('∧'|'∨'|','|'$'|'^'|'_'|'!'|'¬'|'∀'|'≡')+
  ;

SLASH:   '/' ;

NAME
  : ID_START ID_CONTINUE*
  ;

TOBEGIN: ':' ;
TOEND:   ';' ;

SEPARATE
  : '|'
  ;


COMMENT
  : '##'  .*? [\r\n] -> skip
  ;
BLOCK_COMMENT
  :   '/*' .*? '*/' -> skip
  ;
WS
  : [ \r\n\t]+ -> skip
  ;
SHEBANG
  : '#' '!' ~('\n'|'\r')* -> channel(HIDDEN)
  ;

fragment NON_ZERO_DIGIT
  : [1-9]
  ;

fragment DIGIT
  : [0-9]
  ;

fragment SIGN
  : ('+' | '-')
  ;

fragment EXPONENT_OR_POINT_FLOAT
  : (SIGN)? ([0-9]+ | POINT_FLOAT) [eE] [+-]? [0-9]+
  | (SIGN)? POINT_FLOAT
  ;

fragment EXPONENT_OR_POINT_UFLOAT
  : ('U'|'u') ([0-9]+ | POINT_FLOAT) [eE] [+-]? [0-9]+
  | ('U'|'u') POINT_FLOAT
  ;

fragment POINT_FLOAT
  : [0-9]* '.' [0-9]+
  | [0-9]+ '.'
  ;

fragment GLOB_PATTERN
  : 'g\'' ('\\' (RN | .) | ~[\\\r\n'])* '\''
  | 'g"'  ('\\' (RN | .) | ~[\\\r\n"])* '"'
  ;

fragment URI_PATTERN
  : 'f\'' ('\\' (RN | .) | ~[\\\r\n'])* '\''
  | 'f"'  ('\\' (RN | .) | ~[\\\r\n"])* '"'
  ;

fragment CMD_PATTERN
  : '!\'' ('\\' (RN | .) | ~[\\\r\n'])* '\''
  | '!"'  ('\\' (RN | .) | ~[\\\r\n"])* '"'
  ;

fragment JSON_PATTERN
  : 'j\'' ('\\' (RN | .) | ~[\\\r\n'])* '\''
  | 'j"'  ('\\' (RN | .) | ~[\\\r\n"])* '"'
  ;

fragment RN
  : '\r'? '\n'
  ;

fragment SHORT_STRING
  : '\'' ('\\' (RN | .) | ~[\\\r\n'])* '\''
  | '"'  ('\\' (RN | .) | ~[\\\r\n"])* '"'
  ;

fragment LONG_STRING
  : '\'\'\'' LONG_STRING_ITEM*? '\'\'\''
  | '"""' LONG_STRING_ITEM*? '"""'
  ;

fragment LONG_STRING_ITEM
  : ~'\\'
  | '\\' (RN | .)
  ;

fragment ID_START
 : ([A-Z]|[a-z]|SLASH)
 | [a-z]
 ;

fragment ID_CONTINUE
 : ID_START
 | [0-9]
 | SLASH
 ;
