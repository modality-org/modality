#!/usr/bin/env node

const path = require('path');

console.log('🧪 Testing WASM Module Loading');
console.log('================================\n');

// Test the WASM module loading
async function testWasmLoading() {
    try {
        console.log('📁 Current directory:', __dirname);
        console.log('📁 Output directory:', path.resolve(__dirname, '../out'));
        
        // Check if WASM files exist
        const fs = require('fs');
        const wasmJsPath = path.resolve(__dirname, '../out/modality_lang.js');
        const wasmBgPath = path.resolve(__dirname, '../out/modality_lang_bg.wasm');
        
        console.log('🔍 Checking WASM files...');
        console.log('  📄 modality_lang.js exists:', fs.existsSync(wasmJsPath));
        console.log('  📄 modality_lang_bg.wasm exists:', fs.existsSync(wasmBgPath));
        
        if (!fs.existsSync(wasmJsPath) || !fs.existsSync(wasmBgPath)) {
            console.error('❌ WASM files not found in output directory!');
            return;
        }
        
        console.log('\n🚀 Attempting to load WASM module...');
        
        // Try to load the module
        const modulePath = path.resolve(__dirname, '../out/modality_lang.js');
        console.log('📄 Module path:', modulePath);
        
        const modalityLang = await import(modulePath);
        console.log('✅ Module loaded successfully!');
        
        // Check if the module has the expected functions
        const module = modalityLang.default || modalityLang;
        console.log('🔧 Available functions:', Object.keys(module).filter(key => typeof module[key] === 'function'));
        
        // Test parsing a simple model
        const testModel = `model test:
  part p1:
    n1 --> n2: +a
    n2 --> n3: +b
`;
        
        console.log('\n🧪 Testing model parsing...');
        console.log('📝 Test model:', testModel);
        
        if (typeof module.parse_model === 'function') {
            const result = module.parse_model(testModel);
            console.log('✅ parse_model function works!');
            console.log('📊 Parse result:', JSON.stringify(result, null, 2));
        } else {
            console.error('❌ parse_model function not found!');
        }
        
        if (typeof module.generate_mermaid_styled === 'function') {
            console.log('✅ generate_mermaid_styled function found!');
        } else {
            console.error('❌ generate_mermaid_styled function not found!');
        }
        
        console.log('\n🎉 WASM module test completed successfully!');
        
    } catch (error) {
        console.error('❌ Failed to load WASM module:', error);
        console.error('📄 Error details:', error.message);
        console.error('📁 Stack trace:', error.stack);
    }
}

testWasmLoading(); 