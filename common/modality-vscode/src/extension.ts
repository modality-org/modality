import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

// The configured path if it is absolute, else the first match on PATH, else
// a build in the repo this extension was linked from.
function findServer(configured: string, extensionPath: string): string | undefined {
    const isFile = (p: string) => {
        try {
            return fs.statSync(p).isFile();
        } catch {
            return false;
        }
    };
    if (path.isAbsolute(configured)) {
        return isFile(configured) ? configured : undefined;
    }
    const exe = process.platform === 'win32' ? `${configured}.exe` : configured;
    const pathDirs = (process.env.PATH ?? '').split(path.delimiter).filter(Boolean);
    let repo: string | undefined;
    try {
        repo = path.join(fs.realpathSync(extensionPath), '..', '..', 'rust', 'target');
    } catch {
        repo = undefined;
    }
    const candidates = [
        ...pathDirs.map((d) => path.join(d, exe)),
        path.join(extensionPath, 'bin', exe),
        ...(repo ? [path.join(repo, 'release', exe), path.join(repo, 'debug', exe)] : []),
    ];
    return candidates.find(isFile);
}

export async function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('modality');
    const lspEnabled = config.get<boolean>('lsp.enabled', true);
    
    if (!lspEnabled) {
        console.log('Modality LSP is disabled');
        return;
    }
    
    // Get the path to the LSP binary
    const configured = config.get<string>('lsp.path', 'modality-lsp');
    const serverPath = findServer(configured, context.extensionPath);
    if (!serverPath) {
        // Highlighting comes from the grammar and works without the server.
        console.log(`Modality LSP not found (${configured}); running without it`);
        return;
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
