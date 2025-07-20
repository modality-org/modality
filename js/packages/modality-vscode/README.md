# Modality Language Support for VS Code

This extension provides language support for the Modality temporal logic language in Visual Studio Code and Cursor.

## Features

- **Enhanced Syntax Highlighting**: Full syntax highlighting for `.modality` files with special emphasis on modal operators and properties
- **Custom Themes**: Built-in "Modality Dark" and "Modality Light" themes optimized for Modality syntax
- **IntelliSense**: Auto-completion for keywords, operators, and syntax elements
- **Hover Information**: Detailed information about language constructs on hover
- **Real-time Validation**: Syntax validation with error highlighting
- **Commands**: 
  - Generate Mermaid diagrams from Modality models
  - Check formulas against models

## Syntax Highlighting

The extension provides enhanced highlighting for:

### Modal Operators
- **Diamond Operators** (`< >`): Highlighted in orange for existential quantification
- **Box Operators** (`[ ]`): Highlighted in orange for universal quantification

### Properties
- **Positive Properties** (`+property`): Highlighted in teal/green for required properties
- **Negative Properties** (`-property`): Highlighted in red for forbidden properties

### Other Elements
- **Transition Arrows** (`-->`): Highlighted in yellow for state transitions
- **Keywords**: Bold blue highlighting for language keywords
- **Comments**: Green highlighting for single-line comments
- **Strings**: Orange highlighting for string literals
- **Variables**: Light blue highlighting for identifiers

## Language Support

The extension recognizes the following Modality language constructs:

### Keywords
- `model` - Define a new model
- `part` - Define a part within a model
- `formula` - Define a temporal logic formula
- `action` - Define an action with properties
- `test` - Define a test case

### Operators
- `-->` - Transition arrow between states
- `+` - Positive property sign (requires the property)
- `-` - Negative property sign (forbids the property)
- `<` `>` - Diamond operators (exists)
- `[` `]` - Box operators (forall)

### Boolean Values
- `true` - Boolean true value
- `false` - Boolean false value

### Logical Operators
- `and` - Logical AND operator
- `or` - Logical OR operator
- `not` - Logical NOT operator

### Temporal Operators
- `when` - When operator
- `also` - Also operator
- `next` - Next operator
- `must` - Must operator
- `can` - Can operator
- `always` - Always operator
- `eventually` - Eventually operator
- `until` - Until operator
- `lfp` - Least fixed point
- `gfp` - Greatest fixed point

## Installation

### Local Development Installation

For development and testing, you can install the extension locally to both VS Code and Cursor:

```bash
# Install dependencies
pnpm install

# Compile and install locally
pnpm run install:local
```

This will install the extension to:
- **VS Code**: `~/.vscode/extensions/` (macOS) or equivalent on other platforms
- **Cursor**: `~/Library/Application Support/Cursor/User/extensions/` (macOS) or equivalent on other platforms

The script automatically detects which editors are available and installs to all found locations.

### Uninstall Local Extension

To remove the locally installed extension from all editors:

```bash
pnpm run uninstall:local
```

## Usage

1. Open a `.modality` file in VS Code or Cursor
2. The extension will automatically activate and provide language support
3. For best highlighting, select "Modality Dark" or "Modality Light" theme:
   - **VS Code**: `Ctrl+Shift+P` → "Preferences: Color Theme" → "Modality Dark" or "Modality Light"
   - **Cursor**: `Cmd+Shift+P` → "Preferences: Color Theme" → "Modality Dark" or "Modality Light"
4. Use `Ctrl+Space` (or `Cmd+Space` on macOS) for auto-completion
5. Hover over language constructs for detailed information
6. Use the command palette to access Modality-specific commands:
   - `Modality: Generate Mermaid Diagram`
   - `Modality: Check Formula`

## Example

```modality
model ExampleModel:
  part p1:
    n1 --> n2: +blue
    n2 --> n3: +green
    n3 --> n1: +red

formula HasBlue: <+blue> true
formula NoRed: [-red] true
formula Complex: <+blue +green> [-red] <+yellow> true
```

## Development

This extension is part of the Modality project. To contribute:

1. Clone the repository
2. Install dependencies: `pnpm install`
3. Compile the extension: `pnpm run compile`
4. Install locally: `pnpm run install:local`
5. Press F5 in VS Code to run the extension in debug mode

### Available Scripts

- `pnpm run compile` - Compile TypeScript to JavaScript
- `pnpm run watch` - Watch for changes and recompile
- `pnpm run lint` - Run ESLint
- `pnpm run test` - Run tests
- `pnpm run install:local` - Install extension locally for testing (VS Code + Cursor)
- `pnpm run uninstall:local` - Remove locally installed extension from all editors

## License

MIT 