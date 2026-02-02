import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('modality');
    const lspEnabled = config.get<boolean>('lsp.enabled', true);
    
    if (!lspEnabled) {
        console.log('Modality LSP is disabled');
        return;
    }
    
    // Get the path to the LSP binary
    let serverPath = config.get<string>('lsp.path', 'modality-lsp');
    
    // If it's a relative path, try to find it
    if (!path.isAbsolute(serverPath)) {
        // Check common locations
        const possiblePaths = [
            serverPath, // PATH lookup
            path.join(context.extensionPath, 'bin', serverPath),
            path.join(context.extensionPath, '..', '..', 'rust', 'target', 'release', serverPath),
            path.join(context.extensionPath, '..', '..', 'rust', 'target', 'debug', serverPath),
        ];
        
        // Just use the configured path and let it fail if not found
        serverPath = possiblePaths[0];
    }
    
    // Server options - run the LSP binary
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            transport: TransportKind.stdio,
        },
        debug: {
            command: serverPath,
            transport: TransportKind.stdio,
        },
    };
    
    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'modality' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.modality'),
        },
        outputChannelName: 'Modality Language Server',
    };
    
    // Create and start the client
    client = new LanguageClient(
        'modality-lsp',
        'Modality Language Server',
        serverOptions,
        clientOptions
    );
    
    // Register restart command
    const restartCommand = vscode.commands.registerCommand('modality.restartServer', async () => {
        if (client) {
            await client.stop();
            await client.start();
            vscode.window.showInformationMessage('Modality Language Server restarted');
        }
    });
    context.subscriptions.push(restartCommand);
    
    // Start the client
    try {
        await client.start();
        console.log('Modality Language Server started');
    } catch (error) {
        console.error('Failed to start Modality Language Server:', error);
        vscode.window.showErrorMessage(
            `Failed to start Modality Language Server. Make sure 'modality-lsp' is installed and in your PATH, or configure 'modality.lsp.path' in settings.`
        );
    }
}

export async function deactivate(): Promise<void> {
    if (client) {
        await client.stop();
    }
}
