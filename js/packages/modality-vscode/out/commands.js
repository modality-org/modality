"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ModalityCommands = void 0;
const vscode = require("vscode");
const path = require("path");
// Import the Rust WASM parser
let modalityLang = null;
async function getModalityLang() {
    var _a;
    if (!modalityLang) {
        try {
            // Try to load the WASM module from the output directory
            const modulePath = path.resolve(__dirname, './modality_lang.js');
            // Use dynamic import with explicit path resolution
            const module = await (_a = modulePath, Promise.resolve().then(() => require(_a)));
            modalityLang = module.default || module;
            // Initialize the WASM module if needed
            if (modalityLang && typeof modalityLang.init === 'function') {
                await modalityLang.init();
            }
        }
        catch (error) {
            console.warn('Failed to load modality-lang WASM module:', error);
            console.warn('Module path attempted:', path.resolve(__dirname, './modality_lang.js'));
            console.warn('Current __dirname:', __dirname);
            return null;
        }
    }
    return modalityLang;
}
class ModalityCommands {
    constructor() { }
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
            const panel = vscode.window.createWebviewPanel('modalityMermaid', `Mermaid: ${fileName}`, vscode.ViewColumn.Beside, {
                enableScripts: true,
                retainContextWhenHidden: true
            });
            // Generate HTML content with Mermaid rendering
            panel.webview.html = this.getWebviewContent(mermaidContent, fileName);
            vscode.window.showInformationMessage('Mermaid diagram generated successfully!');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Failed to generate Mermaid diagram: ${error}`);
        }
    }
    async visualizeModel(modelNameArg) {
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
                throw new Error('Failed to load modality-lang WASM module');
            }
            // Use parse_all_models to get all models
            const models = lang.parse_all_models(content);
            let modelToShow = null;
            if (Array.isArray(models)) {
                if (modelNameArg) {
                    modelToShow = models.find((m) => m.name === modelNameArg);
                }
                if (!modelToShow) {
                    modelToShow = models[0];
                }
            }
            else {
                modelToShow = models;
            }
            if (!modelToShow) {
                throw new Error('No model found to visualize.');
            }
            const mermaidContent = lang.generate_mermaid(JSON.stringify(modelToShow));
            const modelName = modelToShow && modelToShow.name ? modelToShow.name : fileName;
            // Create a webview panel to display the rendered Mermaid diagram
            const panel = vscode.window.createWebviewPanel('modalityVisualization', `Model: ${modelName}`, vscode.ViewColumn.Beside, {
                enableScripts: true,
                retainContextWhenHidden: true
            });
            // Generate HTML content with Mermaid rendering
            panel.webview.html = this.getWebviewContent(mermaidContent, modelName);
            vscode.window.showInformationMessage(`Model '${modelName}' visualized successfully!`);
        }
        catch (error) {
            // Show a webview panel with the error details
            const errorPanel = vscode.window.createWebviewPanel('modalityVisualizationError', 'Modality Visualization Error', vscode.ViewColumn.Beside, {
                enableScripts: false,
                retainContextWhenHidden: true
            });
            const err = error;
            errorPanel.webview.html = `
                <!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="UTF-8">
                    <meta name="viewport" content="width=device-width, initial-scale=1.0">
                    <title>Modality Visualization Error</title>
                    <style>
                        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #fff; color: #c00; padding: 2em; }
                        h1 { color: #c00; }
                        pre { background: #f8d7da; color: #721c24; border: 1px solid #f5c6cb; border-radius: 4px; padding: 1em; overflow-x: auto; }
                    </style>
                </head>
                <body>
                    <h1>Visualization Failed</h1>
                    <p>The model could not be visualized due to the following error:</p>
                    <pre>${(err && err.stack) ? err.stack : (err && err.message) ? err.message : String(err)}</pre>
                </body>
                </html>
            `;
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
            }
            else {
                vscode.window.showInformationMessage('No formulas found in the model');
            }
        }
        catch (error) {
            vscode.window.showErrorMessage(`Failed to check formula: ${error}`);
        }
    }
    getWebviewContent(mermaidContent, title) {
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
exports.ModalityCommands = ModalityCommands;
//# sourceMappingURL=commands.js.map