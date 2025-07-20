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

// Create the extension directory name
function getExtensionDirName() {
    const packageJson = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'));
    return `${packageJson.publisher}.${packageJson.name.replace('@modality-dev/', '')}`;
}

// Main uninstall function
function uninstallLocal() {
    try {
        console.log('🗑️  Uninstalling Modality VS Code extension...');
        
        // Get paths
        const extensionsDir = getVSCodeExtensionsDir();
        const extensionDirName = getExtensionDirName();
        const targetDir = path.join(extensionsDir, extensionDirName);
        
        if (!extensionsDir) {
            console.log('❌ Could not find VS Code extensions directory.');
            console.log('💡 The extension may not be installed or VS Code may not be configured.');
            process.exit(1);
        }
        
        console.log(`📁 Extensions directory: ${extensionsDir}`);
        console.log(`📁 Target directory: ${targetDir}`);
        
        // Check if the extension is installed
        if (!fs.existsSync(targetDir)) {
            console.log('❌ Extension not found at the expected location.');
            console.log('💡 The extension may have been uninstalled already or installed in a different location.');
            process.exit(1);
        }
        
        // Remove the extension directory
        console.log('🗑️  Removing extension...');
        fs.rmSync(targetDir, { recursive: true, force: true });
        
        console.log('✅ Extension uninstalled successfully!');
        console.log('\n💡 You may need to restart VS Code for the changes to take effect.');
        
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