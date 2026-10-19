" Vim syntax file
" Language:    Bund
" Filenames:   *.bund, and scripts whose first line runs bund or bund2
"
" The rules follow the reference grammar, reference/bund_language_parser/
" bund.pest, as Bund2's lexer reads it (crates/bund2-syntax):
"
"   - a name, an :atom, a `pointer, an @stack and a : or ; command end at
"     whitespace, so `{ 1 println}` is an error: `}` is not whitespace;
"   - numbers, "strings" and 'literals' need no space after them;
"   - a // comment starts only where a new token could start, so the //
"     in `1 2 +// add` belongs to the name `+//` (F61), and is not a comment;
"   - an atom runs to the next whitespace, whatever it holds (F63).

if exists('b:current_syntax')
  finish
endif

let s:cpo_save = &cpo
set cpo&vim

" A word is any run of the characters a Bund name may hold: letters,
" digits and  ! # % & * + , - . / < = > ? ^ _ |  (bund.pest, element).
syn iskeyword @,48-57,192-255,33,35,37,38,42,43,44,45,46,47,60,61,62,63,94,95,124

" Strings may span lines, so look further back than the default.
syn sync minlines=200

" ---------------------------------------------------------------------------
" Words. Every name bund2 registers is a builtin; the groups below pick some
" out for their own colour. A keyword defined later wins, so the groups come
" after the generated list.

exe 'source' fnameescape(expand('<sfile>:p:h') . '/bund_words.vim')

syn keyword bundConditional if if. if.false if.false.in_workbench if.in_workbench if.stack
syn keyword bundConditional ifthenelse ifthenelse. notifthenelse notifthenelse. ?ifthenelse
syn keyword bundConditional ?true ?true. ?true* ?true*. ?false ?false. ?false* ?false*.
syn keyword bundConditional ?try ? ?.

syn keyword bundRepeat while while. loop loop. *loop *loop. times times. for for. do do.
syn keyword bundRepeat map map. fold

syn keyword bundStatement ! !! !. execute execute. apply return return_from return_to
syn keyword bundStatement exit raise context context. endcontext

syn keyword bundDefine register unregister alias alias= alias=. unalias
syn keyword bundDefine lambda lambda* lambda! lambda!. lambda= lambda=.
syn keyword bundDefine var var- var? var?. use use. bootstrap curry
syn keyword bundDefine class object resolve resolve.class resolve.class.

syn keyword bundStackOp dup dup_one dup_one_in dup_many dup_many_in
syn keyword bundStackOp drop drop_in drop_stack drop_stacks swap swap_one swap_in
syn keyword bundStackOp clear clear_in clear_stacks take . move move_from
syn keyword bundStackOp to_stack to_current current stack stack_exists ensure_stack
syn keyword bundStackOp ensure_stack_with_capacity stacks_left stacks_right fold_stack
syn keyword bundStackOp rotate_current_left rotate_current_right
syn keyword bundStackOp rotate_stack_left rotate_stack_right <- <-- -> -->

syn keyword bundOperator + +. - -. * *. / /. ** **. ++ ++. +++ +++.
syn keyword bundOperator *+ *+. *- *-. */ */. == ==. != !=. < <. > >. <= <=. >= >=.
syn keyword bundOperator and and. or or. not not. \|

syn keyword bundBoolean true false True False
syn keyword bundConstant nodata Pi

" The aliases spelled outside ASCII, which cannot be keywords.
syn match bundDefine   '\%(^\|[[:space:]\[{(]\)\@1<=[λΛ]\ze\%(\s\|$\)'
syn match bundStackOp  '\%(^\|[[:space:]\[{(]\)\@1<=[←→]\ze\%(\s\|$\)'
syn match bundOperator '\%(^\|[[:space:]\[{(]\)\@1<=[≠⩽⩾Σ]\.\=\ze\%(\s\|$\)'
syn match bundConstant '\%(^\|[[:space:]\[{(]\)\@1<=[∅π]\ze\%(\s\|$\)'

" ---------------------------------------------------------------------------
" Values.

" 42, -7, +3; 0 alone, since 007 is three integers (F50).
syn match bundNumber '\%(^\|[[:space:]\[{(]\)\@1<=[+-]\=\%(0\|[1-9]\d*\)\ze\%([[:space:]\]})]\|$\)'
" 3.14, 1., 2.5e10, 1.e-3
syn match bundFloat  '\%(^\|[[:space:]\[{(]\)\@1<=[+-]\=\%(0\|[1-9]\d*\)\.\%(\d\+\%([eE][+-]\=\%(0\|[1-9]\d*\)\)\=\|[eE][+-]\=\%(0\|[1-9]\d*\)\)\=\ze\%([[:space:]\]})]\|$\)'
" 1_000: the grammar admits it and the parser refuses it (F49).
syn match bundError  '\%(^\|[[:space:]\[{(]\)\@1<=[+-]\=\d[0-9.]*_[0-9_.eE+-]*\ze\%([[:space:]\]})]\|$\)'

syn region bundString  start=+"+ skip=+\\\\\|\\"+ end=+"+ contains=bundEscape,bundFormatKey,@Spell
syn match  bundEscape  contained '\\\%([btnfr"\\/]\|u\x\{4}\|U\x\{8}\)'
" {name} in a string is a key `format` fills.
syn match  bundFormatKey contained '{[^{}[:space:]"]\+}'
syn region bundLiteral start=+'+ end=+'+

" :name is an atom, a name given as a value; it runs to whitespace (F63).
syn match bundAtom      '\%(^\|[[:space:]\[{(]\)\@1<=:[^[:space:]]\+'
" : and ; alone, or in a run, are commands. Defined after the atom, so `::`
" is a command, as the grammar tries command before atom.
syn match bundCommand   '\%(^\|[[:space:]\[{(]\)\@1<=[:;]\+\ze\%(\s\|$\)'
" @name switches the current stack.
syn match bundStackName '\%(^\|[[:space:]\[{(]\)\@1<=@\%([[:alpha:]]\|[^\x00-\x7f]\)\+\ze\%(\s\|$\)'
" `name is a pointer to a word.
syn match bundPointer   '\%(^\|[[:space:]\[{(]\)\@1<=`[^[:space:]]\+'

" ---------------------------------------------------------------------------
" Structure.

syn match bundLambdaDelim '[{}]'
syn match bundListDelim   '[\[\]]'
syn match bundCtxDelim    '[()]'

" A closing bracket straight after a name, as in `{ 1 println}`: a name must
" end at whitespace. A number or a string may touch it, so those are spared.
syn match bundError +\%([^[:space:]"'\[\]{}()0-9]\)\@1<=[}\])]+

syn keyword bundTodo contained TODO FIXME XXX NOTE
syn match bundComment '\%(^\|[[:space:]\[{(]\)\@1<=//.*$' contains=bundTodo,@Spell
syn match bundShebang '\%^#!.*$'

" ---------------------------------------------------------------------------

hi def link bundBuiltin     Function
hi def link bundConditional Conditional
hi def link bundRepeat      Repeat
hi def link bundStatement   Statement
hi def link bundDefine      Define
hi def link bundStackOp     Special
hi def link bundOperator    Operator
hi def link bundBoolean     Boolean
hi def link bundConstant    Constant
hi def link bundNumber      Number
hi def link bundFloat       Float
hi def link bundString      String
hi def link bundLiteral     String
hi def link bundEscape      SpecialChar
hi def link bundFormatKey   Special
hi def link bundAtom        Identifier
hi def link bundCommand     Keyword
hi def link bundStackName   Label
hi def link bundPointer     Tag
hi def link bundLambdaDelim Delimiter
hi def link bundListDelim   Delimiter
hi def link bundCtxDelim    Delimiter
hi def link bundComment     Comment
hi def link bundTodo        Todo
hi def link bundShebang     PreProc
hi def link bundError       Error

let b:current_syntax = 'bund'

let &cpo = s:cpo_save
unlet s:cpo_save
