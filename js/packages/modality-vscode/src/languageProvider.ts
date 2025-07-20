import * as vscode from 'vscode';

export class ModalityLanguageProvider implements vscode.CompletionItemProvider {
    provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
        context: vscode.CompletionContext
    ): vscode.ProviderResult<vscode.CompletionItem[] | vscode.CompletionList<vscode.CompletionItem>> {
        const linePrefix = document.lineAt(position).text.substr(0, position.character);
        const items: vscode.CompletionItem[] = [];

        // Keywords
        const keywords = [
            { label: 'model', detail: 'Define a new model', documentation: 'Defines a new model with the given name' },
            { label: 'part', detail: 'Define a part within a model', documentation: 'Defines a part within a model' },
            { label: 'formula', detail: 'Define a temporal logic formula', documentation: 'Defines a temporal logic formula' },
            { label: 'action', detail: 'Define an action', documentation: 'Defines an action with properties' },
            { label: 'test', detail: 'Define a test case', documentation: 'Defines a test case' },
            { label: 'true', detail: 'Boolean true value', documentation: 'Boolean true value' },
            { label: 'false', detail: 'Boolean false value', documentation: 'Boolean false value' },
            { label: 'and', detail: 'Logical AND operator', documentation: 'Logical AND operator' },
            { label: 'or', detail: 'Logical OR operator', documentation: 'Logical OR operator' },
            { label: 'not', detail: 'Logical NOT operator', documentation: 'Logical NOT operator' },
            { label: 'when', detail: 'When operator', documentation: 'When operator for temporal logic' },
            { label: 'also', detail: 'Also operator', documentation: 'Also operator for temporal logic' },
            { label: 'next', detail: 'Next operator', documentation: 'Next operator for temporal logic' },
            { label: 'must', detail: 'Must operator', documentation: 'Must operator for temporal logic' },
            { label: 'can', detail: 'Can operator', documentation: 'Can operator for temporal logic' },
            { label: 'always', detail: 'Always operator', documentation: 'Always operator for temporal logic' },
            { label: 'eventually', detail: 'Eventually operator', documentation: 'Eventually operator for temporal logic' },
            { label: 'until', detail: 'Until operator', documentation: 'Until operator for temporal logic' },
            { label: 'lfp', detail: 'Least fixed point', documentation: 'Least fixed point operator' },
            { label: 'gfp', detail: 'Greatest fixed point', documentation: 'Greatest fixed point operator' }
        ];

        // Add keyword completions
        keywords.forEach(keyword => {
            const item = new vscode.CompletionItem(keyword.label, vscode.CompletionItemKind.Keyword);
            item.detail = keyword.detail;
            item.documentation = new vscode.MarkdownString(keyword.documentation);
            items.push(item);
        });

        // Add transition arrow completion
        if (linePrefix.trim().match(/[a-zA-Z_][a-zA-Z0-9_]*\s*$/)) {
            const arrowItem = new vscode.CompletionItem('-->', vscode.CompletionItemKind.Operator);
            arrowItem.detail = 'Transition arrow';
            arrowItem.documentation = new vscode.MarkdownString('Transition arrow between states');
            arrowItem.insertText = '-->';
            items.push(arrowItem);
        }

        // Add property signs
        if (linePrefix.trim().match(/.*-->\s*[a-zA-Z_][a-zA-Z0-9_]*\s*:\s*$/)) {
            const plusItem = new vscode.CompletionItem('+', vscode.CompletionItemKind.Operator);
            plusItem.detail = 'Positive property sign';
            plusItem.documentation = new vscode.MarkdownString('Positive property sign (requires the property)');
            plusItem.insertText = '+';
            items.push(plusItem);

            const minusItem = new vscode.CompletionItem('-', vscode.CompletionItemKind.Operator);
            minusItem.detail = 'Negative property sign';
            minusItem.documentation = new vscode.MarkdownString('Negative property sign (forbids the property)');
            minusItem.insertText = '-';
            items.push(minusItem);
        }

        // Add modal operators
        const modalOperators = [
            { label: '<', detail: 'Diamond operator (exists)', documentation: 'Diamond operator for existential quantification' },
            { label: '>', detail: 'Diamond operator (exists)', documentation: 'Diamond operator for existential quantification' },
            { label: '[', detail: 'Box operator (forall)', documentation: 'Box operator for universal quantification' },
            { label: ']', detail: 'Box operator (forall)', documentation: 'Box operator for universal quantification' }
        ];

        modalOperators.forEach(operator => {
            const item = new vscode.CompletionItem(operator.label, vscode.CompletionItemKind.Operator);
            item.detail = operator.detail;
            item.documentation = new vscode.MarkdownString(operator.documentation);
            items.push(item);
        });

        // Add comment completion
        if (linePrefix.trim() === '') {
            const commentItem = new vscode.CompletionItem('//', vscode.CompletionItemKind.Snippet);
            commentItem.detail = 'Comment';
            commentItem.documentation = new vscode.MarkdownString('Single line comment');
            commentItem.insertText = '// ';
            items.push(commentItem);
        }

        return new vscode.CompletionList(items, false);
    }
} 