# Modality VS Code Extension

VS Code extension for the Modality temporal logic language, providing syntax highlighting, model visualization, and formula checking.

## Features

- **Syntax Highlighting**: Full support for Modality syntax with custom themes
- **Model Visualization**: Generate Mermaid diagrams from Modality models
- **CodeLens Integration**: "Visualize" button appears over model declaration lines
- **Formula Checking**: Basic validation of temporal logic formulas
- **Hover Information**: Context-sensitive help for Modality keywords
- **Real-time Validation**: Live syntax checking as you type

## Installation

### Local Development Installation

```bash
# Install dependencies
pnpm install

# Install extension locally to VS Code and Cursor
pnpm run install:local

# Uninstall extension
pnpm run uninstall:local
```

## Usage

### Model Visualization

The extension provides multiple ways to visualize your Modality models:

1. **CodeLens Button** (Easiest): Click the "Visualize" button that appears above model declaration lines
2. **Visualize Model** (Recommended): Enhanced visualization with model parsing
   - Right-click in a `.modality` file → "Visualize Model"
   - Or use Command Palette: `Modality: Visualize Model`

3. **Generate Mermaid Diagram**: Basic diagram generation
   - Right-click in a `.modality` file → "Generate Mermaid Diagram"
   - Or use Command Palette: `Modality: Generate Mermaid Diagram`

### Example Model

```modality
model SimpleModel:

part StateMachine:
    idle --> active: +start
    active --> processing: +request
    processing --> active: +response
    processing --> idle: +timeout
    active --> idle: +stop

part Controller:
    init --> running: +boot
    running --> paused: +pause
    paused --> running: +resume
    running --> stopped: +shutdown
```

### Formula Checking

- Right-click in a `.modality` file → "Check Formula"
- Or use Command Palette: `Modality: Check Formula`

## Themes

The extension includes custom themes optimized for Modality syntax:

- **Modality Dark**: Dark theme with blue diamond operators and orange box operators
- **Modality Light**: Light theme with consistent coloring

To use the themes:
1. Open Command Palette (`Cmd+Shift+P`)
2. Type "Preferences: Color Theme"
3. Select "Modality Dark" or "Modality Light"

## Syntax Highlighting

The extension provides comprehensive syntax highlighting for:

- **Modal Operators**: `< >` (diamond) and `[ ]` (box) operators
- **Properties**: `+property` and `-property` syntax
- **Named Identifiers**: Model names, part names, formula names, action names, test names
- **Transitions**: `-->` arrows with property labels
- **Keywords**: `model`, `part`, `formula`, `action`, `test`, etc.

## Testing

```bash
# Test syntax highlighting patterns
pnpm run test-syntax

# Open visual test file
pnpm run test-visual

# Test model visualization
pnpm run test-visualization

# Test CodeLens detection
pnpm run test-codelens
```

### Test Files

- `sample-model.modality` - Basic model with transitions and properties
- `multi-model-test.modality` - Multiple models to test CodeLens buttons
- `test-syntax.modality` - Syntax highlighting test cases
- `diamond-test.modality` - Modal operator highlighting tests

## Development

```bash
# Compile TypeScript
pnpm run compile

# Watch for changes
pnpm run watch
```

## Commands

| Command | Description | Shortcut |
|---------|-------------|----------|
| `Modality: Visualize Model` | Generate enhanced Mermaid diagram | Right-click → "Visualize Model" or CodeLens button |
| `Modality: Generate Mermaid Diagram` | Generate basic Mermaid diagram | Right-click → "Generate Mermaid Diagram" |
| `Modality: Check Formula` | Validate temporal logic formulas | Right-click → "Check Formula" |

## File Structure

```
modality-vscode/
├── src/
│   ├── extension.ts          # Main extension entry point
│   ├── commands.ts           # Command implementations
│   └── languageProvider.ts   # Language features
├── syntaxes/
│   └── modality.tmLanguage.json  # TextMate grammar
├── themes/
│   ├── modality-dark.json    # Dark theme
│   └── modality-light.json   # Light theme
├── scripts/
│   ├── install-local.js      # Local installation script
│   ├── uninstall-local.js    # Uninstall script
│   ├── test-syntax.js        # Syntax testing
│   └── visual-test.js        # Visual testing
└── test-syntax.modality      # Syntax test file
``` 