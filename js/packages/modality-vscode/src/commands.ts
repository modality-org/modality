import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

export class ModalityCommands {
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
            // For now, we'll create a simple Mermaid diagram
            // In a real implementation, you would use the modality-lang parser
            const mermaidContent = this.generateSimpleMermaid(content, fileName);
            
            // Create a new document with the Mermaid content
            const mermaidDocument = await vscode.workspace.openTextDocument({
                content: mermaidContent,
                language: 'mermaid'
            });

            await vscode.window.showTextDocument(mermaidDocument, vscode.ViewColumn.Beside);
            vscode.window.showInformationMessage('Mermaid diagram generated successfully!');
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to generate Mermaid diagram: ${error}`);
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
            // For now, we'll do basic validation
            // In a real implementation, you would use the modality-lang parser
            const validationResult = this.validateModalityContent(content);
            
            if (validationResult.isValid) {
                vscode.window.showInformationMessage('Formula validation passed!');
            } else {
                vscode.window.showErrorMessage(`Validation failed: ${validationResult.error}`);
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to check formula: ${error}`);
        }
    }

    private generateSimpleMermaid(content: string, fileName: string): string {
        const lines = content.split('\n');
        let mermaidContent = `graph TD\n`;
        let nodeCounter = 0;
        const nodes = new Set<string>();

        for (const line of lines) {
            const trimmedLine = line.trim();
            
            // Skip comments and empty lines
            if (trimmedLine.startsWith('//') || !trimmedLine) {
                continue;
            }

            // Parse transitions
            const transitionMatch = trimmedLine.match(/^\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*-->\s*([a-zA-Z_][a-zA-Z0-9_]*)/);
            if (transitionMatch) {
                const fromNode = transitionMatch[1];
                const toNode = transitionMatch[2];
                
                if (!nodes.has(fromNode)) {
                    nodes.add(fromNode);
                    mermaidContent += `    ${fromNode}[${fromNode}]\n`;
                }
                
                if (!nodes.has(toNode)) {
                    nodes.add(toNode);
                    mermaidContent += `    ${toNode}[${toNode}]\n`;
                }

                // Extract properties if present
                const propertiesMatch = trimmedLine.match(/:\s*([+-][a-zA-Z_][a-zA-Z0-9_]*(\s*[+-][a-zA-Z_][a-zA-Z0-9_]*)*)/);
                const properties = propertiesMatch ? propertiesMatch[1] : '';
                
                mermaidContent += `    ${fromNode} -->|${properties}| ${toNode}\n`;
            }
        }

        return mermaidContent;
    }

    private validateModalityContent(content: string): { isValid: boolean; error?: string } {
        const lines = content.split('\n');
        let hasModel = false;
        let hasPart = false;
        let hasTransition = false;

        for (const line of lines) {
            const trimmedLine = line.trim();
            
            // Skip comments and empty lines
            if (trimmedLine.startsWith('//') || !trimmedLine) {
                continue;
            }

            // Check for model declaration
            if (trimmedLine.match(/^model\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                hasModel = true;
            }
            // Check for part declaration
            else if (trimmedLine.match(/^part\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                hasPart = true;
            }
            // Check for transition
            else if (trimmedLine.match(/^\s*[a-zA-Z_][a-zA-Z0-9_]*\s*-->\s*[a-zA-Z_][a-zA-Z0-9_]*/)) {
                hasTransition = true;
            }
            // Check for formula declaration
            else if (trimmedLine.match(/^formula\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                // Valid formula declaration
            }
            // Check for action declaration
            else if (trimmedLine.match(/^action\s+[a-zA-Z_][a-zA-Z0-9_]*\s*:/)) {
                // Valid action declaration
            }
            // Check for test declaration
            else if (trimmedLine.match(/^test\s*:/)) {
                // Valid test declaration
            }
            else {
                return { isValid: false, error: `Invalid syntax: ${trimmedLine}` };
            }
        }

        if (!hasModel) {
            return { isValid: false, error: 'No model declaration found' };
        }

        if (!hasPart) {
            return { isValid: false, error: 'No part declaration found' };
        }

        if (!hasTransition) {
            return { isValid: false, error: 'No transitions found' };
        }

        return { isValid: true };
    }
} 