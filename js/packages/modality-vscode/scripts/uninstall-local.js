#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

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
    
    return null;
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
    
    return null;
}

// Create the extension directory name
function getExtensionDirName() {
    const packageJson = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'));
    return `${packageJson.publisher}.${packageJson.name.replace('@modality-dev/', '')}`;
}

// Uninstall from a specific editor
function uninstallFromEditor(editorName, extensionsDir, extensionDirName) {
    const targetDir = path.join(extensionsDir, extensionDirName);
    
    console.log(`\n🗑️  Uninstalling from ${editorName}...`);
    console.log(`📁 Extensions directory: ${extensionsDir}`);
    console.log(`📁 Target directory: ${targetDir}`);
    
    if (!extensionsDir) {
        console.log(`❌ Could not find ${editorName} extensions directory.`);
        return false;
    }
    
    // Check if the extension is installed
    if (!fs.existsSync(targetDir)) {
        console.log(`❌ Extension not found in ${editorName} at the expected location.`);
        return false;
    }
    
    // Remove the extension directory
    console.log('🗑️  Removing extension...');
    fs.rmSync(targetDir, { recursive: true, force: true });
    
    console.log(`✅ Successfully uninstalled from ${editorName}!`);
    return true;
}

// Main uninstall function
function uninstallLocal() {
    try {
        console.log('🗑️  Uninstalling Modality VS Code extension...');
        
        // Get paths
        const vsCodeExtensionsDir = getVSCodeExtensionsDir();
        const cursorExtensionsDir = getCursorExtensionsDir();
        const extensionDirName = getExtensionDirName();
        
        let uninstalledFromAny = false;
        
        // Uninstall from VS Code
        if (vsCodeExtensionsDir) {
            const vsCodeUninstalled = uninstallFromEditor('VS Code', vsCodeExtensionsDir, extensionDirName);
            if (vsCodeUninstalled) uninstalledFromAny = true;
        }
        
        // Uninstall from Cursor
        if (cursorExtensionsDir) {
            const cursorUninstalled = uninstallFromEditor('Cursor', cursorExtensionsDir, extensionDirName);
            if (cursorUninstalled) uninstalledFromAny = true;
        }
        
        if (!uninstalledFromAny) {
            console.log('\n❌ Extension not found in any editor.');
            console.log('💡 The extension may have been uninstalled already or installed in a different location.');
            process.exit(1);
        }
        
        console.log('\n✅ Uninstall completed successfully!');
        console.log('\n💡 You may need to restart your editor(s) for the changes to take effect.');
        
    } catch (error) {
        console.error('❌ Uninstall failed:', error.message);
        process.exit(1);
    }
}

// Run the uninstall
if (require.main === module) {
    uninstallLocal();
}

module.exports = { uninstallLocal }; 