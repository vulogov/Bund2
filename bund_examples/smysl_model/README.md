# smysl_model

A SMYSL library formed and questioned by a model, from Bund, with the model
words of RFC-0014's Phase 2: why the harbour silted up.

1. **A document, ingested and committed.** `smysl.ingest` sends a
   paragraph to the model and answers the units it extracted as a *staged*
   value, writing nothing. The program reads it, then `smysl.commit`
   checks it again against the library under the locks and writes it.
2. **A question, answered and committed.** `smysl.answer` assembles the
   dossier `smysl.dossier` would for the question, asks the model to
   answer it, and stages the dossier and the answer together. Committed,
   the answer is attested at the Model rung, and is no stronger than
   `inferred`. A dossier is built around an entity the question names,
   through the units that mention it, and `smysl.ingest` makes neither. So
   the program writes them with Phase 1's words: an `x.text/entity` for the
   harbour, whose source is the log's alias, and an `x.text/mentions` edge
   to it from each unit section 1 wrote.
3. **A second document**, ingested and committed into the same library.
4. **A text, window by window.** `smysl.text.add` reads a Markdown text
   into the library, and `smysl.ingest.text` sends it to the model one
   window at a time. It commits and journals each window as it goes, so
   when it is run a second time it finds every window journalled and calls
   the model for none.

## Running it

The model words exist only in a `bund2` built with `smysl-model`, and reach
Ollama only with its mapper, `smysl-ollama`:

    make            # build that bund2, then run demo.bund

or, with such a `bund2` on the PATH, from this directory:

    rm -rf harbour && bund2 demo.bund

It needs an Ollama listening at `http://127.0.0.1:11434` that has the model
`llama3.2` (`ollama pull llama3.2`). That is SMYSL's own default, used when
there is no configuration. To use another model, add `:model "<name>" set`
to the `:local` MAP at the top of `demo.bund`, or write a configuration
(below).

**A model's answers are not fixed.** Each run prints different units, so
there is no `expected.txt`. A small model may stage a batch that is not
`clean`, a label without its `/` for instance, which `smysl.commit` would
refuse with SM03; the demo's `commit-clean` commits only a clean batch,
and otherwise prints `not committed` and goes on. With `llama3.2`, the
documents staged clean on every run made for this example. The question
did not on one run, whose labels lacked the `/`. The text's passes came
back as JSON cut short, and SMYSL committed each window as opaque prose
(`SMY-W304`, counted as `degraded`). SMYSL's own measurements say the same
of `llama3.2` and `qwen2.5:14b-instruct` over a real corpus.

A test in `crates/bund2-cli/tests`, `smysl_model.rs`, runs this demo in a
fresh directory and asserts that every step returned and every batch was
committed. The test is `#[ignore]`d, because CI has no Ollama, and is run
by hand:

    cargo test -p bund2-cli --features smysl-ollama --test smysl_model -- --ignored

## What each word sends

Every model word sends some of the library's content to the provider that
SMYSL routes the `content-ingest` task to:

| Word | Sends |
|---|---|
| `smysl.ingest` | its `input` text |
| `smysl.answer` | each dossier member's role, uid, status and gist, the question first |
| `smysl.ingest.text` | the text's windows, and a digest of what earlier windows extracted |
| `smysl.commit` | nothing: it writes, and calls no model |

A prompt override (`ingest.prompt` in the configuration) is sent too,
along with the text of every `*_file` it names, wherever on the host that
file is.

Before calling the model, each word probes the provider. A provider that
cannot be reached, or an Ollama that does not have the model, is SM13, and
nothing is sent, staged or written. A hosted provider's model is not
checked against its list, which leaves out names it accepts (DeepSeek's
`deepseek-chat`, for one); a model it does not take fails at the first
call instead.

## Keeping it local: `offline`

`opts.offline` set to `true` is SMYSL's `--offline`. A provider whose
endpoint is not a loopback address is refused, SM13, before any socket is
opened. Whether an endpoint is local is decided from its host, so a
configuration cannot claim it. Every call in `demo.bund` passes it.

Without `offline`, the words go wherever the configuration routes them.
With no configuration, that is the local Ollama above: SMYSL's default
never reaches a hosted provider.

## The configuration

The words read SMYSL's own configuration, found as SMYSL's commands find
it:

- `opts.config`, an absolute path, when given;
- otherwise `.smysl/config.hjson` in the directory that holds the library's
  root. For this demo's `harbour`, that is `.smysl/config.hjson` here.
- When neither exists, SMYSL's default: one Ollama at
  `http://127.0.0.1:11434`, model `llama3.2`, with every task routed to
  it.

For example:

    {
      providers: {
        ollama: {
          kind: ollama
          endpoint: "http://127.0.0.1:11434"
          model: "qwen2.5"
          context_window: 8192
          max_output: 2048
          structured: json-schema
        }
      }
      routing: { content-ingest: ollama }
      fallback: [ollama]
    }

A configuration never holds a key: only `api_key_env`, the name of an
environment variable, or `api_key_cmd`, a command that prints one. SMYSL
reads the key when a hosted provider is called, which for `api_key_cmd`
means running the command once per model call and once per probe.

## Proxies are not used

Model calls go directly to the provider's endpoint. SMYSL's HTTP client
reads no proxy variable, so `http_proxy` and `https_proxy` are ignored,
although Bund2's own `url` honours `http_proxy`. A provider that can be
reached only through a proxy cannot be reached from these words.

## Ollama over `https://`

`smysl-ollama` links an HTTP client without TLS, so that it builds without
a C compiler. An `ollama` provider at an `https://` endpoint therefore fails
as unreachable, SM13, in a build with only that mapper. A build that also
has a hosted mapper (`smysl-anthropic`, `smysl-openai`, `smysl-gemini` or
`smysl-deepseek`) has TLS, and reaches it. Those mappers need a C
compiler, for `ring`.

## Locks

`smysl.commit` takes the library's locks for its one write, as Phase 1's
words do. `smysl.ingest` and `smysl.answer` hold no lock while the model
works: they only read the library.

`smysl.ingest.text` holds the locks for the whole run, which takes a model
call per window and pass. While it runs, any other write to that library
from this process is refused at once with SM07, "the library is locked:
bund2 smysl.ingest.text is writing it in this process", instead of waiting.
A writer in another process finds SMYSL's lock and is refused too. If a
window's commit fails part way, the locks are kept for an operator to
inspect, as for any failed write (`bund2 smysl-unlock`). The windows
already committed stay committed, and their journal lets a later run
resume.
