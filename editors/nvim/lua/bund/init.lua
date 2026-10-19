-- Bund support for Neovim: indentation, running a file, and `bund2 check`.
local M = {}

local function program()
  return vim.g.bund2_program or 'bund2'
end

-- The line with everything that cannot hold a bracket taken out: strings,
-- 'literals', comments, and :atoms, `pointers and @stacks, which run to the
-- next whitespace. A string that spans lines is not followed.
local function brackets_only(line)
  local out, i, n = {}, 1, #line
  local boundary = true -- could a new token start here?
  while i <= n do
    local c = line:sub(i, i)
    if c == '"' then
      local j = i + 1
      while j <= n do
        local d = line:sub(j, j)
        if d == '\\' then
          j = j + 2
        elseif d == '"' then
          break
        else
          j = j + 1
        end
      end
      i, boundary = j + 1, false
    elseif c == "'" then
      i, boundary = (line:find("'", i + 1, true) or n) + 1, false
    elseif boundary and line:sub(i, i + 1) == '//' then
      break
    elseif boundary and (c == ':' or c == '`' or c == '@') then
      i = line:find('%s', i) or n + 1
    else
      if c:match('[{}%[%]()]') then
        out[#out + 1] = c
      end
      boundary = c:match('[%s{%[(]') ~= nil
      i = i + 1
    end
  end
  return table.concat(out)
end

-- How many closing brackets a line starts with, spaces between them allowed.
local function leading_closers(line)
  local run = line:match('^[%s}%])]*') or ''
  return select(2, run:gsub('[}%])]', ''))
end

-- How deep to indent line `lnum`: the indent of the line above, one level in
-- for each bracket it leaves open, one level out for each bracket this line
-- starts by closing.
function M.indent(lnum)
  lnum = lnum or vim.v.lnum
  local prev = vim.fn.prevnonblank(lnum - 1)
  if prev == 0 then
    return 0
  end
  local raw = vim.fn.getline(prev)
  local above = brackets_only(raw)
  local opened = select(2, above:gsub('[{%[(]', '')) - select(2, above:gsub('[}%])]', ''))
  -- Closers the line above starts with were taken off its own indent already.
  local levels = opened + leading_closers(raw) - leading_closers(vim.fn.getline(lnum))
  return math.max(0, vim.fn.indent(prev) + levels * vim.fn.shiftwidth())
end

-- Run the file in a terminal split. The buffer is written first, because
-- bund2 reads the file.
function M.run(args)
  local file = vim.api.nvim_buf_get_name(0)
  if file == '' then
    vim.notify('BundRun: the buffer has no file', vim.log.levels.ERROR)
    return
  end
  if vim.fn.executable(program()) == 0 then
    vim.notify('BundRun: cannot run ' .. program() .. '; set g:bund2_program', vim.log.levels.ERROR)
    return
  end
  vim.cmd('silent update')
  local cmd = vim.fn.shellescape(program()) .. ' ' .. vim.fn.shellescape(file)
  if args and args ~= '' then
    cmd = cmd .. ' ' .. args
  end
  vim.cmd('botright new')
  if vim.fn.has('nvim-0.11') == 1 then
    vim.fn.jobstart(cmd, { term = true })
  else
    vim.fn.termopen(cmd)
  end
  vim.cmd('startinsert')
end

local ns = vim.api.nvim_create_namespace('bund_check')

-- What `bund2 check` prints, as diagnostics. Two shapes:
--   Warning: <file>:<line>:<col>: <message>     a stack that can underflow
--   line <line>: <message>                       a parse error
function M.parse_check(output, bufnr)
  local found = {}
  for _, l in ipairs(vim.split(output, '\n', { plain = true })) do
    local _, lnum, col, msg = l:match('^Warning: (.-):(%d+):(%d+): (.*)$')
    if lnum then
      found[#found + 1] = {
        bufnr = bufnr, lnum = tonumber(lnum) - 1, col = tonumber(col) - 1,
        severity = vim.diagnostic.severity.WARN, message = msg, source = 'bund2 check',
      }
    else
      local plnum, pmsg = l:match('^line (%d+): (.*)$')
      if plnum then
        found[#found + 1] = {
          bufnr = bufnr, lnum = tonumber(plnum) - 1, col = 0,
          severity = vim.diagnostic.severity.ERROR, message = pmsg, source = 'bund2 check',
        }
      end
    end
  end
  return found
end

function M.check()
  local bufnr = vim.api.nvim_get_current_buf()
  local file = vim.api.nvim_buf_get_name(bufnr)
  if file == '' or vim.fn.filereadable(file) == 0 then
    vim.notify('BundCheck: write the buffer to a file first', vim.log.levels.WARN)
    return
  end
  if vim.fn.executable(program()) == 0 then
    vim.notify('BundCheck: cannot run ' .. program() .. '; set g:bund2_program', vim.log.levels.ERROR)
    return
  end
  vim.system({ program(), 'check', '--file', file }, { text = true }, function(res)
    vim.schedule(function()
      if not vim.api.nvim_buf_is_valid(bufnr) then
        return
      end
      local out = (res.stdout or '') .. (res.stderr or '')
      local found = M.parse_check(out, bufnr)
      vim.diagnostic.set(ns, bufnr, found)
      vim.fn.setqflist({}, 'r', {
        title = 'bund2 check',
        items = vim.tbl_map(function(d)
          return { bufnr = d.bufnr, lnum = d.lnum + 1, col = d.col + 1, text = d.message,
                   type = d.severity == vim.diagnostic.severity.ERROR and 'E' or 'W' }
        end, found),
      })
      local summary = out:match('(checked [^\n]*)') or (#found .. ' finding(s)')
      vim.notify(summary:gsub('^checked .-: ', 'bund2 check: '), vim.log.levels.INFO)
    end)
  end)
end

return M
