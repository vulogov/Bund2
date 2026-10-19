# Bund for Neovim

Syntax highlighting, filetype detection, indentation and commands for Bund
programs. It works with Neovim 0.10 or later. It needs no plugins and no
tree-sitter parser, and works with any colour scheme.

What you get in a `.bund` file:

- **Colour for every part of the grammar:** words, numbers, strings, `'literals'`,
  `:atoms`, `@stacks`, `` `pointers ``, the `:` and `;` commands, `{ }`
  lambdas, `[ ]` lists, `( )` contexts and `//` comments.
- **Every word `bund2` registers is known**, 657 of them, including the
  workbench `.` forms and the `,` forms. Control flow, definitions, stack
  shuffling, operators and constants each get their own colour.
- **Mistakes the parser would refuse are marked**, as you type them.
- **`{name}` keys inside a string are picked out**, for `format`.
- **Brackets indent by two spaces**, as the examples do.
- **`gc` comments a line out with `//`.** `w`, `*` and `ciw` take
  `math.sqrt` or `graph.path.` as one word.
- **`:BundRun`** runs the file in a terminal split. **`:BundCheck`** shows
  what `bund2 check` finds as diagnostics, and `:make` does the same through
  the quickfix list.

## Installing

Pick one. Each makes this directory part of Neovim's runtime path.

**Neovim's own package directory**, from this directory:

    make install            # links it into ~/.local/share/nvim/site/pack/bund/start
    make uninstall          # removes the link

**lazy.nvim**, in your plugin list:

```lua
{ dir = '/path/to/Bund2/editors/nvim', name = 'bund', lazy = false }
```

Keep `lazy = false`. The plugin is a few small files, and filetype
detection has to be loaded before the first `.bund` file opens.

**By hand**, in `init.lua`:

```lua
vim.opt.runtimepath:append('/path/to/Bund2/editors/nvim')
```

**Copying the files** into `~/.config/nvim/` works too. Keep the layout:
`syntax/bund.vim` and `syntax/bund_words.vim`, `ftdetect/bund.lua`,
`ftplugin/bund.lua`, `indent/bund.lua` and `lua/bund/init.lua`.

Then open a Bund file and check:

    :set filetype?          " filetype=bund

A file is detected as Bund by its `.bund` extension, or when its first line
runs Bund: `#!/usr/bin/env bund2`, or the same with `bund`.

The two commands need `bund2` on your `PATH`. Point them at another binary
with:

```lua
vim.g.bund2_program = '/path/to/Bund2/target/release/bund2'
```

## What the colours mean

Each Bund group links to a standard highlight group, so your colour scheme
decides the actual colour.

| group | links to | what it marks |
|---|---|---|
| `bundBuiltin` | `Function` | any word `bund2` registers: `println`, `math.sqrt`, `graph.path.` |
| `bundConditional` | `Conditional` | `if`, `ifthenelse`, `?true`, `?try`, `?` … |
| `bundRepeat` | `Repeat` | `while`, `loop`, `times`, `for`, `do`, `map`, `fold` |
| `bundStatement` | `Statement` | `!`, `execute`, `apply`, `return`, `exit`, `raise`, `context` … |
| `bundDefine` | `Define` | `register`, `alias`, `lambda`, `var`, `use`, `class`, `object`, `λ` … |
| `bundStackOp` | `Special` | `dup`, `drop`, `swap`, `take`, `.`, `move`, `<-`, `→` … |
| `bundOperator` | `Operator` | `+`, `*`, `==`, `<=`, `and`, `not`, `≠`, `Σ` … |
| `bundBoolean` | `Boolean` | `true`, `false`, `True`, `False` |
| `bundConstant` | `Constant` | `nodata`, `∅`, `Pi`, `π` |
| `bundNumber`, `bundFloat` | `Number`, `Float` | `42`, `-7`, `3.14`, `2.5e10` |
| `bundString` | `String` | `"text"` |
| `bundFormatKey` | `Special` | `{name}` inside a string |
| `bundEscape` | `SpecialChar` | `\n`, `\"`, `é` inside a string |
| `bundLiteral` | `String` | `'text'` |
| `bundAtom` | `Identifier` | `:name` |
| `bundCommand` | `Keyword` | `:` and `;` on their own |
| `bundStackName` | `Label` | `@work` |
| `bundPointer` | `Tag` | `` `println `` |
| `bundLambdaDelim`, `bundListDelim`, `bundCtxDelim` | `Delimiter` | `{ }`, `[ ]`, `( )` |
| `bundComment`, `bundTodo` | `Comment`, `Todo` | `// …`, and `TODO`, `FIXME`, `XXX`, `NOTE` in it |
| `bundShebang` | `PreProc` | `#!/usr/bin/env bund2` |
| `bundError` | `Error` | something the parser refuses (below) |

A word the program defines with `register` keeps the plain text colour,
because it is not in `bund2`'s list.

To change a colour, set the group after your colour scheme loads:

```lua
vim.api.nvim_create_autocmd('ColorScheme', {
  callback = function()
    vim.api.nvim_set_hl(0, 'bundAtom', { link = 'Constant' })
    vim.api.nvim_set_hl(0, 'bundStackOp', { fg = '#d08770', bold = true })
  end,
})
```

## What it reads the way the parser does

The rules follow Bund's grammar, `reference/bund_language_parser/bund.pest`,
as Bund2 reads it. A few of them surprise people, and the colours show them
as they are:

- **A name must end at whitespace.** `{ 1 println}` does not parse,
  because `}` is not whitespace, so that `}` is marked as an error. A number
  or a string may touch a bracket: `[1.0 10.0]` is fine.
- **`//` starts a comment only where a new word could start.** In
  `1 2 +// add`, `+//` is one name (F61), and `add` is another. Neither is
  a comment, so neither is coloured as one.
- **An atom runs to the next whitespace**, whatever it holds (F63). In
  `[ :A :B]`, the atom is `B]`, and it is coloured to the end.
- **`1_000` is marked as an error.** The grammar admits the underscore,
  and the parser then fails to convert it (F49).
- **`007` is three integers** (F50), so it is not coloured as a number.

Strings may span lines. Highlighting looks back 200 lines to find where one
began.

## Commands

| command | does |
|---|---|
| `:BundRun [args]` | writes the buffer, then runs `bund2 <file> [args]` in a terminal split |
| `:BundCheck` | runs `bund2 check --file <file>` and shows what it finds as diagnostics, also filling the quickfix list |
| `:make` | the same check, through `makeprg` and `errorformat`; a parse error is not listed this way, so use `:BundCheck` |

`bund2 check` reports where the stack can run short, without running the
program. Its warnings appear as warning diagnostics, and a parse error
appears as an error diagnostic. To check every time the file is written,
set this before opening a Bund file:

```lua
vim.g.bund_check_on_save = true
```

Some keys you might add, in `~/.config/nvim/after/ftplugin/bund.lua`:

```lua
vim.keymap.set('n', '<leader>r', '<cmd>BundRun<cr>', { buffer = true, desc = 'Run Bund file' })
vim.keymap.set('n', '<leader>c', '<cmd>BundCheck<cr>', { buffer = true, desc = 'Check Bund file' })
```

## Buffer settings

The filetype plugin sets these for Bund buffers only:

| option | value | why |
|---|---|---|
| `commentstring` | `// %s` | `gc` comments with `//` |
| `iskeyword` | letters, digits and `! # % & * + , - . / < = > ? ^ _ \|` | a word is what a Bund name may hold |
| `shiftwidth`, `softtabstop` | 2 | as the examples indent |
| `expandtab` | on | spaces, not tabs |
| `makeprg` | `bund2 check --file %` | `:make` checks the file |

Override any of them in `~/.config/nvim/after/ftplugin/bund.lua`.

## Keeping the word list current

`syntax/bund_words.vim` is generated from what `bund2 words` prints. Its
header names the `bund2` version it came from. When Bund2 gains words,
regenerate it:

    make words                              # uses ../../target/release/bund2
    make words BUND2=$(command -v bund2)    # or any other binary

Words outside ASCII cannot be Vim keywords, so the generator writes them as
patterns bounded by whitespace.

## Testing

    make test

This runs Neovim headless over `test/fixture.bund` and checks the
following:
- the highlight group at 42 places in the fixture;
- indentation of a bracketed block;
- that filetype detection reads the shebang;
- that `bund2 check`'s output becomes diagnostics.

It prints `ok: …` and exits 0, or names each failure and exits 1.

## Files

    ftdetect/bund.lua         .bund files and bund shebangs
    ftplugin/bund.lua         buffer settings, :BundRun, :BundCheck
    indent/bund.lua           bracket indentation
    lua/bund/init.lua         the indent function, run and check
    syntax/bund.vim           the highlighting rules
    syntax/bund_words.vim     every word bund2 registers (generated)
    tools/generate-words.sh   writes syntax/bund_words.vim
    test/                     the headless checks and their fixture
