import * as vscode from 'vscode';
import { ModalityLanguageProvider } from './languageProvider';
import { ModalityCommands } from './commands';

export function activate(context: vscode.ExtensionContext) {
    console.log('Modality extension is now active!');

    // Register language provider
    const languageProvider = new ModalityLanguageProvider();
    const providerRegistration = vscode.languages.registerCompletionItemProvider(
        { language: 'modality' },
        languageProvider
    );

    // Register commands
    const commands = new ModalityCommands();
    
    const generateMermaidCommand = vscode.commands.registerCommand(
        'modality.generateMermaid',
        commands.generateMermaid.bind(commands)
    );

    const visualizeModelCommand = vscode.commands.registerCommand(
        'modality.visualizeModel',
        commands.visualizeModel.bind(commands)
    );

    const checkFormulaCommand = vscode.commands.registerCommand(
        'modality.checkFormula',
        commands.checkFormula.bind(commands)
    );

    // Register CodeLens provider for model visualization
    const codeLensProvider = vscode.languages.registerCodeLensProvider(
        { language: 'modality' },
        {
            provideCodeLenses(document, token) {
                const codeLenses: vscode.CodeLens[] = [];
                const text = document.getText();
                const lines = text.split('\n');

                for (let i = 0; i < lines.length; i++) {
                    const line = lines[i];
                    const trimmedLine = line.trim();
                    
                    // Check for model declaration
                    const modelMatch = trimmedLine.match(/^model\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*:/);
                    if (modelMatch) {
                        const modelName = modelMatch[1];
                        const range = new vscode.Range(i, 0, i, line.length);
                        
                        const codeLens = new vscode.CodeLens(range, {
                            title: 'Visualize',
                            command: 'modality.visualizeModel',
                            arguments: []
                        });
                        
                        codeLenses.push(codeLens);
                    }
                }

                return codeLenses;
            }
        }
    );

    // Register hover provider for syntax help
    const hoverProvider = vscode.languages.registerHoverProvider(
        { language: 'modality' },
        {
            provideHover(document, position, token) {
                const range = document.getWordRangeAtPosition(position);
                const word = document.getText(range);
                
                const hoverInfo = getHoverInfo(word);
                if (hoverInfo) {
                    return new vscode.Hover(hoverInfo);
                }
                
                return null;
            }
        }
    );

    // Register diagnostic collection
    const diagnosticCollection = vscode.languages.createDiagnosticCollection('modality');
    context.subscriptions.push(diagnosticCollection);

    // Register document change listener for real-time validation
    const changeListener = vscode.workspace.onDidChangeTextDocument(event => {
        if (event.document.languageId === 'modality') {
            validateDocument(event.document, diagnosticCollection);
        }
    });

    // Register document open listener for initial validation
    const openListener = vscode.workspace.onDidOpenTextDocument(document => {
        if (document.languageId === 'modality') {
            validateDocument(document, diagnosticCollection);
        }
    });

    context.subscriptions.push(
        providerRegistration,
        generateMermaidCommand,
        visualizeModelCommand,
        checkFormulaCommand,
        codeLensProvider,
        hoverProvider,
        diagnosticCollection,
        changeListener,
        openListener
    );
}

function getHoverInfo(word: string): vscode.MarkdownString | null {
    const hoverInfo: { [key: string]: string } = {
        'model': 'Defines a new model with the given name',
        'part': 'Defines a part within a model',
        'formula': 'Defines a temporal logic formula',
        'action': 'Defines an action with properties',
        'test': 'Defines a test case',
        '-->': 'Transition arrow between states',
        'true': 'Boolean true value',
        'false': 'Boolean false value',
        'and': 'Logical AND operator',
        'or': 'Logical OR operator',
        'not': 'Logical NOT operator',
        '<': 'Diamond operator (exists)',
        '>': 'Diamond operator (exists)',
        '[': 'Box operator (forall)',
        ']': 'Box operator (forall)',
        '+': 'Positive property sign',
        '-': 'Negative property sign'
    };

    const info = hoverInfo[word.toLowerCase()];
    if (info) {
        const markdown = new vscode.MarkdownString();
        markdown.appendCodeblock(word, 'modality');
        markdown.appendMarkdown(`\n\n${info}`);
        return markdown;
    }

    return null;
}

function validateDocument(document: vscode.TextDocument, collection: vscode.DiagnosticCollection) {
    const diagnostics: vscode.Diagnostic[] = [];
    const text = document.getText();
    const lines = text.split('\n');

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const lineNumber = i + 1;

        // Basic syntax validation
        if (line.trim() && !line.trim().startsWith('//')) {
            // Check for model declaration
            if (line.match(/^model\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                // Valid model declaration
            } else if (line.match(/^part\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                // Valid part declaration
            } else if (line.match(/^formula\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                // Valid formula declaration
            } else if (line.match(/^action\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                // Valid action declaration
            } else if (line.match(/^test\s*:/)) {
                // Valid test declaration
            } else if (line.match(/^\s*[a-zA-Z_][a-zA-Z0-9_]*\s*-->\s*[a-zA-Z_][a-zA-Z0-9_]*/)) {
                // Valid transition
            } else if (line.trim()) {
                // Invalid syntax
                const range = new vscode.Range(lineNumber - 1, 0, lineNumber - 1, line.length);
                const diagnostic = new vscode.Diagnostic(
                    range,
                    'Invalid Modality syntax',
                    vscode.DiagnosticSeverity.Error
                );
                diagnostics.push(diagnostic);
            }
        }
    }

    collection.set(document.uri, diagnostics);
}

export function deactivate() {
    console.log('Modality extension is now deactivated!');
} 