# Modality Language BNF Grammar

This document provides the formal Backus-Naur Form (BNF) grammar specification for the Modality temporal logic language.

## Complete BNF Grammar

```
<file> ::= <comment>* <model> <comment>*

<model> ::= "model" <identifier> ":" <graph>*

<graph> ::= "graph" <identifier> ":" <transition>*

<transition> ::= <identifier> "-->" <identifier> [":" <property-list>]

<property-list> ::= <property> [<property-list>]

<property> ::= <property-sign> <identifier>

<property-sign> ::= "+" | "-"

<identifier> ::= <letter> [<identifier-char>]

<identifier-char> ::= <letter> | <digit> | "_"

<letter> ::= "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" |
            "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"

<digit> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"

<comment> ::= "//" <any-char>* <newline>

<any-char> ::= <any-character-except-newline>

<newline> ::= "\n" | "\r" | "\r\n"
```

## Grammar Elements

### Terminals
- `"model"` - Keyword to start a model definition
- `"graph"` - Keyword to start a graph definition  
- `"-->"` - Arrow operator for transitions
- `":"` - Colon separator
- `"+"` - Positive property sign
- `"-"` - Negative property sign
- `"//"` - Comment start
- `<letter>` - Alphabetic characters (a-z, A-Z)
- `<digit>` - Numeric characters (0-9)
- `"_"` - Underscore character
- `<newline>` - Line ending characters

### Non-terminals
- `<file>` - Complete Modality file
- `<model>` - Model definition with name and graphs
- `<graph>` - Graph definition with name and transitions
- `<transition>` - Transition between two nodes with optional properties
- `<property-list>` - List of properties (zero or more)
- `<property>` - Single property with sign and name
- `<property-sign>` - Property sign (+ or -)
- `<identifier>` - Valid identifier name
- `<identifier-char>` - Valid character for identifiers
- `<comment>` - Single-line comment
- `<any-char>` - Any character except newline
- `<newline>` - Line ending

## Grammar Rules Explained

### File Structure
```
<file> ::= <comment>* <model> <comment>*
```
A file consists of optional comments, followed by a model, followed by optional comments.

### Model Definition
```
<model> ::= "model" <identifier> ":" <graph>*
```
A model starts with the keyword "model", followed by an identifier, a colon, and zero or more graphs.

### Graph Definition
```
<graph> ::= "graph" <identifier> ":" <transition>*
```
A graph starts with the keyword "graph", followed by an identifier, a colon, and zero or more transitions.

### Transition Definition
```
<transition> ::= <identifier> "-->" <identifier> [":" <property-list>]
```
A transition consists of two identifiers separated by "-->" (three dashes followed by greater than), optionally followed by a colon and property list.

### Property List
```
<property-list> ::= <property> [<property-list>]
```
A property list is a recursive definition allowing one or more properties.

### Property
```
<property> ::= <property-sign> <identifier>
```
A property consists of a sign (+ or -) followed by an identifier.

### Identifier
```
<identifier> ::= <letter> [<identifier-char>]
<identifier-char> ::= <letter> | <digit> | "_"
```
An identifier starts with a letter and can contain letters, digits, and underscores.

## Examples

### Valid Syntax Examples

**Simple transition:**
```
n1 --> n2
```

**Transition with single property:**
```
n1 --> n2: +blue
```

**Transition with multiple properties:**
```
n1 --> n2: +blue -red +green
```

**Complete model:**
```
model MyModel:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: -red
```

**Model with multiple graphs:**
```
model ComplexModel:
  graph g1:
    n1 --> n2: +blue
    n2 --> n3: -red
  graph g2:
    n1 --> n1: +yellow
```

**File with comments:**
```
// This is a comment
model CommentedModel:
  graph g1:
    n1 --> n2: +blue  // Another comment
    n2 --> n3: -red
```

### Invalid Syntax Examples

**Invalid identifier (starts with digit):**
```
1node --> n2  // Invalid
```

**Missing colon after model name:**
```
model MyModel  // Missing colon
  graph g1:
```

**Invalid property sign:**
```
n1 --> n2: *blue  // Invalid sign
```

**Missing property name:**
```
n1 --> n2: +  // Missing property name
```

**Wrong arrow syntax:**
```
n1 -> n2  // Should be -->
```

## Implementation Notes

- **Whitespace**: Spaces and tabs are ignored between tokens
- **Comments**: Single-line comments start with `//` and continue to end of line
- **Case Sensitivity**: Identifiers are case-sensitive
- **Multiple Models**: Files can contain multiple models (parsed separately)
- **Empty Lists**: Graphs and property lists can be empty
- **Recursion**: Property lists use left recursion for efficient parsing
- **Arrow Syntax**: Transitions use `-->` (three dashes followed by greater than) 