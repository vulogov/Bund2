# Features

- Generator/Funtion and Operator and more clearly defined
- Prefunction is introduced for all datatypes.
- Function N, generating sequence of number in a range. Example: 20 10 N
- Dynamic loadable modules in Go supported. Example: ./internal/dynmodules/example
- Function 'use' to load dynamic compiled modules. Example: 'internal/dynmodules/example/example.so' use
- Function I for generating interval, I/Add to add another intervals and I/StdDev and I/Coeff to test if number is within or outside of intervals.
- "Execution" of "unix" and "http" types with "!" supported. String containing content of the output or document is placed on the stack.


# Bugfixes

# Removed features

- <letter>'string' notation in favor to the prefunction notation: <prefunction name>@<data item> and unlikely letter-string, prefunction is available for all data types.
