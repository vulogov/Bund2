-- *.bund files, and scripts whose first line runs bund or bund2:
--   #!/usr/bin/env bund2
vim.filetype.add({
  extension = { bund = 'bund' },
  pattern = {
    ['.*'] = {
      function(_, bufnr)
        local first = vim.api.nvim_buf_get_lines(bufnr, 0, 1, false)[1] or ''
        if first:match('^#!.*%f[%w]bund2?%f[^%w]') then
          return 'bund'
        end
      end,
      { priority = -math.huge },
    },
  },
})
