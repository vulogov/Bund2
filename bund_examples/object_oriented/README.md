# object_oriented

Every part of Bund's object system, over shapes, a duck, a few small
class trees and the built-in classes:
- defining and registering a class, and making objects from it;
- fields and methods;
- constructors, and the order they run in;
- inheritance, multiple inheritance included, where the first match wins
  and a diamond is searched depth first;
- a method written once that asks each object for its own answer;
- the base classes every object can reach;
- the classes that wrap one value;
- the words Bund2 adds, and the mistakes a program can catch.

    make            # run demo.bund
    make twice      # run it twice and compare the output

or, without `make`, from this directory:

    bund2 demo.bund

Nothing it prints changes from run to run. An object's `.id` and
`.timestamp` do change, so the demo compares them rather than printing
them.

## A class

A class is a dict of slots. A slot that holds a lambda is a **method**;
any other slot is a **field**:

    :Rect class                          // start a class
      :.super [ :Object ] set            // its parents, by name
      :w 1.0 set                         // a field, with its default
      :h 1.0 set
      :.init { swap :h swap set swap :w swap set } set   // the constructor
      :area  { dup dup :w get swap :h get * } set        // a method
    register                             // file it under the name beneath

    :door 1.0 2.0 :Rect object var       // make one: the constructor takes 1.0 and 2.0

- **`class` leaves the name where it is**, and `register` takes it from
  beneath the class. That is why the idiom is `:Name class … register`.
- **A method is called as `<name> <object> !`.** The name goes beneath
  the object. `!` dispatches on the object and leaves it on the stack, so
  the method finds it on top. `:area :door var? !` leaves the door with its
  area above it.
- **A field is read the same way.** `:h :door var? !` pushes the field's
  value, and `:door var? :w get` reads it as from any dict.
- **A value never changes.** `set` on an object answers a changed copy,
  and the door kept in `:door` is still the door it was.
- **`.init` is the constructor.** It runs with the new object on top and
  the arguments beneath it. If it leaves an object of the class on top,
  that object is used in place of the one it was given, which is how a
  constructor fills in fields. If it leaves anything else, the object is
  used as it was built, and what the constructor left stays on the stack.

## Inheritance

`.super` lists a class's parents by name. A slot an object lacks is
looked for in its parents, **depth first, in the order `.super` lists
them**, and the first one found wins. A slot the class has itself always
comes first.

- **One method, three answers.** `Shape` has a `describe` method that
  asks the object for its `area`. `Square` and `Circle` each have their own
  `area`, and `Blob` has none, so it gets `Shape`'s 0.0:

      square of area 9.0
      circle of area 12.56636
      blob of area 0.0

- **Multiple inheritance.** A `Duck` is a `Swimmer` and then a `Flyer`.
  Both say how it moves, and the `Swimmer` is listed first, so it swims.
  The wings come from the `Flyer`, and the quack is the Duck's own.
- **A diamond is searched depth first, not nearest first.** `Left` and
  `Right` both descend from `Base`, and only `Right` overrides `greet`.
  With `Left` listed first, `Left` is searched all the way up to `Base`
  before `Right` is looked at, so `Base` answers:

      Left listed first:  hello from Base
      Right listed first: hello from Right

  Python's rule (C3) would answer `Right`'s in both. The order is pinned
  against the reference by `tests/probes/oop-multiple-inheritance.bund`.
- **Constructors run parents first.** `A`'s parents are `B` and `E`, and
  `B`'s are `C` and `D`. Each constructor prints its class:

      C D B E A

## What every object inherits

Bund's base classes form a chain: `Object` → `Printable` → `Display`. A
class with `Object` in its `.super` reaches all three:

| class | slot | does |
|---|---|---|
| `Object` | `.id` | the object's identity, a string; two objects never share one |
| `Object` | `.timestamp` | when the object was made, a FLOAT |
| `Printable` | `str` | the wrapped value as a string, or `Object(<class>)` |
| `Printable` | `print`, `println` | print what `str` gives |
| `Display` | `format` | fill the class's `.template`, taking any key the object lacks from the stack |
| `Display` | `display` | `format`, then print it |

Each leaves the object on the stack. `.id`, `.timestamp`, `str` and
`format` put their answer above it.

## Objects that wrap one value

| class | made by | keeps |
|---|---|---|
| `Value` | `<v> :Value object` | `v` as given, in `.data` |
| `Integer`, `Float`, `Bool` | `<v> :Integer object`, … | `v` converted: `"42" :Integer object` holds 42 |
| `True`, `False` | the words `True` and `False` | a Bool object |
| `List` | `List`, or `<list> :List object` | a list; `push` adds to it |
| `Floats` | `Floats`, … | a list that makes each item a FLOAT |
| `Intervals` | `Intervals`, or `<pairs> :Intervals object` | ranges; `overlap` names the one a number falls in |

These words work on any of them:

| word | does |
|---|---|
| `unwrap` | replace the object with its value |
| `is` | answer its value and keep the object beneath |
| `wrap` | `<value> <object> wrap`: the object with a new value in it |
| `#` | `<object> <lambda> #`: run the lambda on the value |

`push` and `overlap` are methods: `"pear" :push :basket var? !` answers
the basket with a pear in it, and `15.0 :overlap :bands var? !` answers
`10..20`.

## What Bund2 adds

- **`<class> !` makes an object from a class value** (D23), however the
  value came to be on the stack: from `resolve.class`, or built on the
  spot and never registered. The class must carry its own `.class_name`
  (D25). Every built-in class does; a class made with `class` does not
  until it is given one, and the demo shows that refusal. In the
  reference, `!` on a class always fails (F16).
- **`?is`** answers whether an object is of a class or of one descended
  from it (D96, RFC-0010). It looks up the tree and never down: a `Floats`
  is a `List`, and a `List` is not a `Floats`. The reference has no such
  word, so no golden can run it.
- **Workbench forms.** `?class.`, `?object.`, `resolve.class.`, `is.`,
  `?is.` and `#.` take their operand from the workbench and leave the
  answer there. `?is.` still takes the class name from the stack.

## What goes wrong

Each of these is an error the demo catches with `?try`:

    a class never registered: … OBJECT class Oval not registered
    a slot no class has:      … VM no method colour has been registered
    a parent never registered: … OBJECT class Nobody not registered
    a misspelled word:        … i(Lsit) for stack returned: Inline Lsit not registered
    ?is on a number:          … ?IS: NO OBJECT IN #2

- **A missing parent is named.** The reference names the child instead,
  `Orphan`, which is registered (F67).
- **A misspelled word is reported in the reference's words** (D197), the
  name it could not find twice over.
- **A failed call leaves its object on the stack.** Dispatch looks at the
  object without taking it, so the demo's handler drops the door.

Every other line the demo prints is what the reference prints for the
same program, less the Rust source paths the reference adds to an error's
text (F66). The exceptions are the words Bund2 adds and the missing
parent's name.

## Writing Bund like this

- `:name <value> var` keeps an object to ask several questions of, and
  `:name var?` brings it back.
- A method that answers something should leave its object beneath the
  answer, as the built-in methods do: `dup dup :w get swap :h get *`.
- A method can call another method on the same object, `:area swap !`,
  and the call reaches the object's own slot first.
- `println` cannot print an object. Ask it for `str`, or for a field.
- `a b >` asks whether `b` is greater than `a`: the top is the left side.
