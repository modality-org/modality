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

// Main installation function
function installLocal() {
    try {
        console.log('🚀 Installing Modality VS Code extension locally...');
        
        // Get paths
        const projectDir = path.resolve(__dirname, '..');
        const extensionsDir = getVSCodeExtensionsDir();
        const extensionDirName = getExtensionDirName();
        const targetDir = path.join(extensionsDir, extensionDirName);
        
        console.log(`📁 Project directory: ${projectDir}`);
        console.log(`📁 Extensions directory: ${extensionsDir}`);
        console.log(`📁 Target directory: ${targetDir}`);
        
        // Check if extensions directory exists, create if it doesn't
        if (!fs.existsSync(extensionsDir)) {
            console.log(`📁 Creating extensions directory: ${extensionsDir}`);
            fs.mkdirSync(extensionsDir, { recursive: true });
            console.log('💡 Note: VS Code extensions directory created. You may need to restart VS Code.');
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
            'syntaxes'
        ];
        
        console.log('📋 Copying files...');
        
        // Copy individual files
        for (const file of filesToCopy) {
            const srcFile = path.join(projectDir, file);
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
            const srcDir = path.join(projectDir, dir);
            const destDir = path.join(targetDir, dir);
            
            if (fs.existsSync(srcDir)) {
                copyDir(srcDir, destDir);
                console.log(`  ✅ ${dir}/`);
            } else {
                console.log(`  ⚠️  ${dir}/ (not found)`);
            }
        }
        
        console.log('\n🎉 Installation completed successfully!');
        console.log(`📦 Extension installed to: ${targetDir}`);
        console.log('\n💡 To use the extension:');
        console.log('   1. Restart VS Code');
        console.log('   2. Open a .modality file');
        console.log('   3. The extension should activate automatically');
        console.log('\n🔧 To uninstall, run:');
        console.log(`   rm -rf "${targetDir}"`);
        
        // Check if VS Code is running and suggest restart
        try {
            if (process.platform === 'darwin') {
                execSync('pgrep -f "Code"', { stdio: 'ignore' });
                console.log('\n⚠️  VS Code appears to be running. Please restart it to load the extension.');
            }
        } catch (e) {
            // VS Code is not running, which is fine
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