-- Indent Bund by its brackets: one level inside each { [ ( left open.
if vim.b.did_indent then
  return
end
vim.b.did_indent = 1

vim.bo.indentexpr = "v:lua.require'bund'.indent()"
vim.bo.indentkeys = '0},0],0),!^F,o,O'
vim.bo.autoindent = true

vim.b.undo_indent = 'setlocal indentexpr< indentkeys< autoindent<'
