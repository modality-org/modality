"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const assert = require("assert");
const vscode = require("vscode");
suite('Modality Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');
    test('Extension should be present', () => {
        assert.ok(vscode.extensions.getExtension('modality-dev.modality-vscode'));
    });
    test('Should activate', async () => {
        const extension = vscode.extensions.getExtension('modality-dev.modality-vscode');
        if (extension) {
            await extension.activate();
            assert.ok(true);
        }
    });
    test('Should register commands', async () => {
        const commands = await vscode.commands.getCommands();
        assert.ok(commands.includes('modality.generateMermaid'));
        assert.ok(commands.includes('modality.checkFormula'));
    });
});
//# sourceMappingURL=index.js.map