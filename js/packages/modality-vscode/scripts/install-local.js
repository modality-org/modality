#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Get the VS Code extensions directory
function getVSCodeExtensionsDir() {
    const homeDir = process.env.HOME || process.env.USERPROFILE;
    
    // Platform-specific paths
    const platform = process.platform;
    const possiblePaths = [];
    
    if (platform === 'darwin') {
        // macOS - ~/.vscode/extensions is the primary path
        possiblePaths.push(
            path.join(homeDir, '.vscode', 'extensions'),
            path.join(homeDir, 'Library', 'Application Support', 'Code', 'User', 'extensions'),
            path.join(homeDir, 'Library', 'Application Support', 'Code - Insiders', 'User', 'extensions')
        );
    } else if (platform === 'win32') {
        // Windows
        possiblePaths.push(
            path.join(homeDir, '.vscode', 'extensions'),
            path.join(homeDir, 'AppData', 'Roaming', 'Code', 'User', 'extensions'),
            path.join(homeDir, 'AppData', 'Roaming', 'Code - Insiders', 'User', 'extensions')
        );
    } else {
        // Linux and others
        possiblePaths.push(
            path.join(homeDir, '.vscode', 'extensions'),
            path.join(homeDir, '.config', 'Code', 'User', 'extensions'),
            path.join(homeDir, '.config', 'Code - Insiders', 'User', 'extensions')
        );
    }
    
    // Find the first existing directory
    for (const dir of possiblePaths) {
        if (fs.existsSync(dir)) {
            return dir;
        }
    }
    
    // If none exist, return the most likely one and let the user create it
    return possiblePaths[0];
}

// Get the Cursor extensions directory
function getCursorExtensionsDir() {
    const homeDir = process.env.HOME || process.env.USERPROFILE;
    
    // Platform-specific paths for Cursor
    const platform = process.platform;
    const possiblePaths = [];
    
    if (platform === 'darwin') {
        // macOS
        possiblePaths.push(
            path.join(homeDir, 'Library', 'Application Support', 'Cursor', 'User', 'extensions'),
            path.join(homeDir, '.cursor', 'extensions')
        );
    } else if (platform === 'win32') {
        // Windows
        possiblePaths.push(
            path.join(homeDir, 'AppData', 'Roaming', 'Cursor', 'User', 'extensions'),
            path.join(homeDir, '.cursor', 'extensions')
        );
    } else {
        // Linux and others
        possiblePaths.push(
            path.join(homeDir, '.config', 'Cursor', 'User', 'extensions'),
            path.join(homeDir, '.cursor', 'extensions')
        );
    }
    
    // Find the first existing directory
    for (const dir of possiblePaths) {
        if (fs.existsSync(dir)) {
            return dir;
        }
    }
    
    // If none exist, return the most likely one and let the user create it
    return possiblePaths[0];
}

// Create the extension directory name
function getExtensionDirName() {
    const packageJson = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'));
    return `${packageJson.publisher}.${packageJson.name.replace('@modality-dev/', '')}`;
}

// Copy directory recursively
function copyDir(src, dest) {
    if (!fs.existsSync(dest)) {
        fs.mkdirSync(dest, { recursive: true });
    }
    
    const entries = fs.readdirSync(src, { withFileTypes: true });
    
    for (const entry of entries) {
        const srcPath = path.join(src, entry.name);
        const destPath = path.join(dest, entry.name);
        
        if (entry.isDirectory()) {
            copyDir(srcPath, destPath);
        } else {
            fs.copyFileSync(srcPath, destPath);
        }
    }
}

// Install to a specific editor
function installToEditor(editorName, extensionsDir, extensionDirName) {
    const targetDir = path.join(extensionsDir, extensionDirName);
    
    console.log(`\n📦 Installing to ${editorName}...`);
    console.log(`📁 Extensions directory: ${extensionsDir}`);
    console.log(`📁 Target directory: ${targetDir}`);
    
    // Check if extensions directory exists, create if it doesn't
    if (!fs.existsSync(extensionsDir)) {
        console.log(`📁 Creating ${editorName} extensions directory: ${extensionsDir}`);
        fs.mkdirSync(extensionsDir, { recursive: true });
        console.log(`💡 Note: ${editorName} extensions directory created. You may need to restart ${editorName}.`);
    }
    
    // Remove existing installation if it exists
    if (fs.existsSync(targetDir)) {
        console.log(`🗑️  Removing existing installation: ${targetDir}`);
        fs.rmSync(targetDir, { recursive: true, force: true });
    }
    
    // Create target directory
    fs.mkdirSync(targetDir, { recursive: true });
    
    // Copy necessary files
    const filesToCopy = [
        'package.json',
        'language-configuration.json',
        'README.md',
        '.vscodeignore'
    ];
    
    const dirsToCopy = [
        'out',
        'syntaxes',
        'themes'
    ];
    
    console.log('📋 Copying files...');
    
    // Copy individual files
    for (const file of filesToCopy) {
        const srcFile = path.join(process.cwd(), file);
        const destFile = path.join(targetDir, file);
        
        if (fs.existsSync(srcFile)) {
            fs.copyFileSync(srcFile, destFile);
            console.log(`  ✅ ${file}`);
        } else {
            console.log(`  ⚠️  ${file} (not found)`);
        }
    }
    
    // Copy directories
    for (const dir of dirsToCopy) {
        const srcDir = path.join(process.cwd(), dir);
        const destDir = path.join(targetDir, dir);
        
        if (fs.existsSync(srcDir)) {
            copyDir(srcDir, destDir);
            console.log(`  ✅ ${dir}/`);
        } else {
            console.log(`  ⚠️  ${dir}/ (not found)`);
        }
    }
    
    console.log(`✅ Successfully installed to ${editorName}!`);
    return targetDir;
}

// Main installation function
function installLocal() {
    try {
        console.log('🚀 Installing Modality VS Code extension locally...');
        
        // Get paths
        const projectDir = path.resolve(__dirname, '..');
        const vsCodeExtensionsDir = getVSCodeExtensionsDir();
        const cursorExtensionsDir = getCursorExtensionsDir();
        const extensionDirName = getExtensionDirName();
        
        console.log(`📁 Project directory: ${projectDir}`);
        
        const installedPaths = [];
        
        // Install to VS Code
        try {
            const vsCodePath = installToEditor('VS Code', vsCodeExtensionsDir, extensionDirName);
            installedPaths.push({ editor: 'VS Code', path: vsCodePath });
        } catch (error) {
            console.log(`⚠️  Failed to install to VS Code: ${error.message}`);
        }
        
        // Install to Cursor
        try {
            const cursorPath = installToEditor('Cursor', cursorExtensionsDir, extensionDirName);
            installedPaths.push({ editor: 'Cursor', path: cursorPath });
        } catch (error) {
            console.log(`⚠️  Failed to install to Cursor: ${error.message}`);
        }
        
        if (installedPaths.length === 0) {
            console.log('❌ Failed to install to any editor.');
            process.exit(1);
        }
        
        console.log('\n🎉 Installation completed successfully!');
        console.log('\n📦 Installed to:');
        installedPaths.forEach(({ editor, path }) => {
            console.log(`  • ${editor}: ${path}`);
        });
        
        console.log('\n💡 To use the extension:');
        console.log('   1. Restart your editor (VS Code or Cursor)');
        console.log('   2. Open a .modality file');
        console.log('   3. The extension should activate automatically');
        console.log('   4. Select "Modality Dark" or "Modality Light" theme for best highlighting');
        console.log('\n🔧 To uninstall, run:');
        console.log(`   pnpm run uninstall:local`);
        
        // Check if editors are running and suggest restart
        try {
            if (process.platform === 'darwin') {
                const vsCodeRunning = execSync('pgrep -f "Code"', { stdio: 'ignore' }).toString().trim();
                if (vsCodeRunning) {
                    console.log('\n⚠️  VS Code appears to be running. Please restart it to load the extension.');
                }
                
                const cursorRunning = execSync('pgrep -f "Cursor"', { stdio: 'ignore' }).toString().trim();
                if (cursorRunning) {
                    console.log('⚠️  Cursor appears to be running. Please restart it to load the extension.');
                }
            }
        } catch (e) {
            // Editors are not running, which is fine
        }
        
    } catch (error) {
        console.error('❌ Installation failed:', error.message);
        process.exit(1);
    }
}

// Run the installation
if (require.main === module) {
    installLocal();
}

module.exports = { installLocal }; 