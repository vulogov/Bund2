-- Buffer settings and commands for Bund.
if vim.b.did_ftplugin then
  return
end
vim.b.did_ftplugin = 1

local opt = vim.opt_local

opt.commentstring = '// %s'
opt.comments = '://'
opt.formatoptions:remove('t')
opt.formatoptions:append('croql')

-- A word is what a Bund name may hold, so `w`, `*` and `ciw` take
-- `math.sqrt`, `?try` and `graph.path.` whole.
opt.iskeyword = '@,48-57,192-255,33,35,37,38,42,43,44,45,46,47,60,61,62,63,94,95,124'

-- The corpus indents by two spaces.
opt.expandtab = true
opt.shiftwidth = 2
opt.softtabstop = 2

-- :make reports what `bund2 check` finds, without running the program.
opt.makeprg = (vim.g.bund2_program or 'bund2') .. ' check --file %'
opt.errorformat = '%WWarning: %f:%l:%c: %m,%-G%.%#'

local bund = require('bund')

vim.api.nvim_buf_create_user_command(0, 'BundRun', function(args)
  bund.run(args.args)
end, { nargs = '*', complete = 'file', desc = 'Run this Bund file with bund2' })

vim.api.nvim_buf_create_user_command(0, 'BundCheck', function()
  bund.check()
end, { desc = 'Show what bund2 check finds as diagnostics' })

if vim.g.bund_check_on_save then
  local group = vim.api.nvim_create_augroup('bund_check_' .. vim.api.nvim_get_current_buf(), {})
  vim.api.nvim_create_autocmd('BufWritePost', {
    group = group,
    buffer = 0,
    callback = function() bund.check() end,
  })
end

vim.b.undo_ftplugin = table.concat({
  'setlocal commentstring< comments< formatoptions< iskeyword<',
  'expandtab< shiftwidth< softtabstop< makeprg< errorformat<',
  '| silent! delcommand -buffer BundRun',
  '| silent! delcommand -buffer BundCheck',
}, ' ')
