import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

// Import the Rust WASM parser
let modalityLang: any = null;

async function getModalityLang() {
    if (!modalityLang) {
        try {
            // Try to load the WASM module from the output directory
            const modulePath = path.resolve(__dirname, './modality_lang.js');
            
            // Use dynamic import with explicit path resolution
            const module = await import(modulePath);
            modalityLang = module.default || module;
            
            // Initialize the WASM module if needed
            if (modalityLang && typeof modalityLang.init === 'function') {
                await modalityLang.init();
            }
        } catch (error) {
            console.warn('Failed to load modality-lang WASM module:', error);
            console.warn('Module path attempted:', path.resolve(__dirname, './modality_lang.js'));
            console.warn('Current __dirname:', __dirname);
            return null;
        }
    }
    return modalityLang;
}

export class ModalityCommands {
    constructor() {}

    async generateMermaid() {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'modality') {
            vscode.window.showErrorMessage('Please open a .modality file first');
            return;
        }

        const document = editor.document;
        const content = document.getText();
        const fileName = path.basename(document.fileName, '.modality');

        try {
            // Use Rust WASM parser for proper stateDiagram-v2 generation
            const lang = await getModalityLang();
            if (!lang) {
                vscode.window.showErrorMessage('Failed to load modality-lang WASM module');
                return;
            }

            const modelJson = lang.parse_model(content);
            const mermaidContent = lang.generate_mermaid(JSON.stringify(modelJson));
            
            // Create a webview panel to display the rendered Mermaid diagram
            const panel = vscode.window.createWebviewPanel(
                'modalityMermaid',
                `Mermaid: ${fileName}`,
                vscode.ViewColumn.Beside,
                {
                    enableScripts: true,
                    retainContextWhenHidden: true
                }
            );

            // Generate HTML content with Mermaid rendering
            panel.webview.html = this.getWebviewContent(mermaidContent, fileName);
            
            vscode.window.showInformationMessage('Mermaid diagram generated successfully!');
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to generate Mermaid diagram: ${error}`);
        }
    }

    async visualizeModel() {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'modality') {
            vscode.window.showErrorMessage('Please open a .modality file first');
            return;
        }

        const document = editor.document;
        const content = document.getText();
        const fileName = path.basename(document.fileName, '.modality');

        try {
            // Use Rust WASM parser for proper stateDiagram-v2 generation
            const lang = await getModalityLang();
            if (!lang) {
                vscode.window.showErrorMessage('Failed to load modality-lang WASM module');
                return;
            }

            const modelJson = lang.parse_model(content);
            const mermaidContent = lang.generate_mermaid(JSON.stringify(modelJson));
            
            // Extract model name from parsed JSON
            const modelName = modelJson && modelJson.name ? modelJson.name : fileName;
            
            // Create a webview panel to display the rendered Mermaid diagram
            const panel = vscode.window.createWebviewPanel(
                'modalityVisualization',
                `Model: ${modelName}`,
                vscode.ViewColumn.Beside,
                {
                    enableScripts: true,
                    retainContextWhenHidden: true
                }
            );

            // Generate HTML content with Mermaid rendering
            panel.webview.html = this.getWebviewContent(mermaidContent, modelName);
            
            vscode.window.showInformationMessage(`Model '${modelName}' visualized successfully!`);
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to visualize model: ${error}`);
        }
    }

    async checkFormula() {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'modality') {
            vscode.window.showErrorMessage('Please open a .modality file first');
            return;
        }

        const document = editor.document;
        const content = document.getText();

        try {
            // Use Rust WASM parser for formula checking
            const lang = await getModalityLang();
            if (!lang) {
                vscode.window.showErrorMessage('Failed to load modality-lang WASM module');
                return;
            }

            const modelJson = lang.parse_model(content);
            
            // Check if parsing was successful
            if (modelJson && modelJson.formulas) {
                const formulaCount = modelJson.formulas.length;
                vscode.window.showInformationMessage(`Found ${formulaCount} formula(s) in the model`);
            } else {
                vscode.window.showInformationMessage('No formulas found in the model');
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to check formula: ${error}`);
        }
    }

    private getWebviewContent(mermaidContent: string, title: string): string {
        return `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>${title} - Mermaid Diagram</title>
                <script src="https://cdn.jsdelivr.net/npm/mermaid@11.9.0/dist/mermaid.min.js"></script>
                <style>
                    body {
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                        margin: 0;
                        padding: 20px;
                        background-color: var(--vscode-editor-background);
                        color: var(--vscode-editor-foreground);
                    }
                    .container {
                        max-width: 1200px;
                        margin: 0 auto;
                    }
                    .header {
                        margin-bottom: 20px;
                        padding-bottom: 10px;
                        border-bottom: 1px solid var(--vscode-panel-border);
                    }
                    .title {
                        font-size: 24px;
                        font-weight: bold;
                        margin-bottom: 10px;
                    }
                    .section {
                        margin-bottom: 30px;
                    }
                    .section-title {
                        font-size: 18px;
                        font-weight: bold;
                        margin-bottom: 10px;
                        color: var(--vscode-textLink-foreground);
                    }
                    .mermaid-container {
                        background: white;
                        border: 1px solid var(--vscode-panel-border);
                        border-radius: 4px;
                        padding: 20px;
                        margin-bottom: 20px;
                        overflow: auto;
                    }
                    .code-container {
                        background: var(--vscode-textBlockQuote-background);
                        border: 1px solid var(--vscode-panel-border);
                        border-radius: 4px;
                        padding: 15px;
                        margin-bottom: 20px;
                    }
                    .code-title {
                        font-weight: bold;
                        margin-bottom: 10px;
                        color: var(--vscode-textPreformat-foreground);
                    }
                    pre {
                        background: var(--vscode-textBlockQuote-background);
                        border: 1px solid var(--vscode-panel-border);
                        border-radius: 4px;
                        padding: 15px;
                        overflow-x: auto;
                        font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
                        font-size: 12px;
                        line-height: 1.4;
                        margin: 0;
                        white-space: pre-wrap;
                        word-wrap: break-word;
                    }
                    .error {
                        color: var(--vscode-errorForeground);
                        background: var(--vscode-inputValidation-errorBackground);
                        border: 1px solid var(--vscode-inputValidation-errorBorder);
                        border-radius: 4px;
                        padding: 15px;
                        margin-bottom: 20px;
                    }
                    .success {
                        color: var(--vscode-notificationsInfoIcon-foreground);
                        background: var(--vscode-notificationsInfoBackground);
                        border: 1px solid var(--vscode-notificationsInfoBorder);
                        border-radius: 4px;
                        padding: 15px;
                        margin-bottom: 20px;
                    }
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <div class="title">${title}</div>
                        <div>Generated Mermaid Diagram</div>
                    </div>

                    <div class="section">
                        <div class="section-title">📊 Rendered Diagram</div>
                        <div class="mermaid-container">
                            <div class="mermaid">
                                ${mermaidContent}
                            </div>
                        </div>
                    </div>

                    <div class="section">
                        <div class="section-title">🔍 Debug Information</div>
                        <div class="code-container">
                            <div class="code-title">Raw Mermaid Code:</div>
                            <pre>${mermaidContent.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</pre>
                        </div>
                    </div>

                    <div class="section">
                        <div class="section-title">📋 Mermaid Version</div>
                        <div class="success">
                            Using Mermaid version 11.9.0
                        </div>
                    </div>
                </div>

                <script>
                    // Define the mermaid content from the server
                    const mermaidContent = \`${mermaidContent.replace(/`/g, '\\`').replace(/\$/g, '\\$')}\`;
                    
                    // Initialize Mermaid
                    mermaid.initialize({
                        startOnLoad: true,
                        theme: 'default',
                        flowchart: {
                            useMaxWidth: true,
                            htmlLabels: true
                        },
                        stateDiagram: {
                            useMaxWidth: true,
                            htmlLabels: true
                        }
                    });

                    // Add error handling
                    mermaid.contentLoaded();
                    
                    // Check for syntax errors
                    try {
                        mermaid.parse(mermaidContent);
                        console.log('Mermaid syntax is valid');
                    } catch (error) {
                        console.error('Mermaid syntax error:', error);
                        const errorDiv = document.createElement('div');
                        errorDiv.className = 'error';
                        errorDiv.innerHTML = '<strong>Syntax Error:</strong><br>' + error.message;
                        document.querySelector('.mermaid-container').appendChild(errorDiv);
                    }
                </script>
            </body>
            </html>
        `;
    }
} 