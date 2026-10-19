-- Headless checks for the Bund syntax, indent and check parser.
--   nvim --headless --clean --cmd 'set rtp^=.' -c 'luafile test/run.lua'
local here = vim.fn.fnamemodify(debug.getinfo(1, 'S').source:sub(2), ':p:h')
local failures = 0

local function check(what, got, want)
  if not vim.deep_equal(got, want) then
    failures = failures + 1
    print(('FAIL %s: got %s, want %s'):format(what, vim.inspect(got), vim.inspect(want)))
  end
end

vim.cmd('edit ' .. vim.fn.fnameescape(here .. '/fixture.bund'))
check('filetype', vim.bo.filetype, 'bund')
vim.cmd('syntax on')

-- The group at the first occurrence of `text` on line `lnum` (plus `off`).
local function group(lnum, text, off)
  local line = vim.fn.getline(lnum)
  local s = line:find(text, 1, true)
  if not s then
    return 'NOT FOUND: ' .. text
  end
  local id = vim.fn.synID(lnum, s + (off or 0), 1)
  return vim.fn.synIDattr(id, 'name')
end

local cases = {
  { 1, '#!', 'bundShebang' },
  { 2, '//', 'bundComment' },
  { 2, 'TODO', 'bundTodo' },
  { 3, ':Rect', 'bundAtom' },
  { 3, 'class', 'bundDefine' },
  { 4, '1.0', 'bundFloat' },
  { 4, 'set', 'bundBuiltin' },
  { 5, '{', 'bundLambdaDelim' },
  { 5, 'dup', 'bundStackOp' },
  { 5, '*', 'bundOperator' },
  { 6, 'register', 'bundDefine' },
  { 7, '42', 'bundNumber' },
  { 7, '-7', 'bundNumber' },
  { 7, '3.14', 'bundFloat' },
  { 7, '2.5e10', 'bundFloat' },
  { 7, '1_000', 'bundError' },
  { 7, '"a', 'bundString' },
  { 7, '{name}', 'bundFormatKey' },
  { 7, '\\"', 'bundEscape' },
  { 7, "'lit'", 'bundLiteral' },
  { 8, '@work', 'bundStackName' },
  { 8, '`print', 'bundPointer' },
  { 8, '::', 'bundCommand' },
  { 8, ';', 'bundCommand' },
  { 8, 'true', 'bundBoolean' },
  { 8, 'nodata', 'bundConstant' },
  { 8, '∅', 'bundConstant' },
  { 8, 'λ', 'bundDefine' },
  { 8, '←', 'bundStackOp' },
  { 8, '≠', 'bundOperator' },
  -- F61: `+//` is one name, so nothing on this line is a comment.
  { 9, '+//', '' },
  { 9, 'add', '' },
  -- A name must end at whitespace.
  { 10, '}', 'bundError' },
  { 10, 'println', 'bundBuiltin' },
  { 11, '[', 'bundListDelim' },
  { 11, ':A', 'bundAtom' },
  { 11, '(', 'bundCtxDelim' },
  { 11, 'if', 'bundConditional' },
  { 11, 'while', 'bundRepeat' },
  { 11, 'math.sqrt', 'bundBuiltin' },
  { 11, 'graph.path.', 'bundBuiltin' },
  { 11, '?try', 'bundConditional' },
}
for _, c in ipairs(cases) do
  check(('line %d %q'):format(c[1], c[2]), group(c[1], c[2], c[4]), c[3])
end

-- Indentation, by brackets.
vim.cmd('enew')
vim.bo.filetype = 'bund'
vim.api.nvim_buf_set_lines(0, 0, -1, false, {
  ':greet {',
  '"hi" println',
  '[ 1',
  '2 ]',
  '} register',
  '{ 1 } map',
  '"{ not a bracket" :x{ println // {',
  'done',
})
vim.cmd('normal! gg=G')
check('indent', vim.api.nvim_buf_get_lines(0, 0, -1, false), {
  ':greet {',
  '  "hi" println',
  '  [ 1',
  '    2 ]',
  '} register',
  '{ 1 } map',
  '"{ not a bracket" :x{ println // {',
  'done',
})

-- What `bund2 check` prints, as diagnostics.
local found = require('bund').parse_check(table.concat({
  'Warning: /x/z.bund:1:3: `+` needs 2 value(s); 1 can be proven here',
  'line 2: expected whitespace or end of input, found `}`',
  'checked /x/z.bund: 1 finding(s), 3 site(s) analysed, 0 abandoned',
}, '\n'), 0)
check('check count', #found, 2)
check('check warning', found[1] and { found[1].lnum, found[1].col, found[1].severity }, { 0, 2, vim.diagnostic.severity.WARN })
check('check parse error', found[2] and { found[2].lnum, found[2].severity }, { 1, vim.diagnostic.severity.ERROR })

-- Comment toggling uses //.
check('commentstring', vim.bo.commentstring, '// %s')

if failures == 0 then
  print(('ok: %d syntax cases, indent, check parser'):format(#cases))
  vim.cmd('qa!')
else
  print(failures .. ' failure(s)')
  vim.cmd('cq!')
end
