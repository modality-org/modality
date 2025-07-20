#!/usr/bin/env node

const path = require('path');

console.log('🔍 Testing Parser Error Handling');
console.log('================================\n');

// Test parser error handling
async function testParserError() {
    try {
        console.log('🚀 Loading WASM module...');
        
        // Load the WASM module
        const modulePath = path.resolve(__dirname, '../out/modality_lang.js');
        const modalityLang = await import(modulePath);
        const module = modalityLang.default || modalityLang;
        
        console.log('✅ Module loaded successfully!');
        
        // Test with invalid model
        const invalidModel = `model Invalid:
  this is not valid syntax
`;
        
        console.log('📝 Invalid test model:', invalidModel);
        
        // Test parsing
        console.log('\n🧪 Testing parse_model with invalid model...');
        try {
            const invalidModelJsonString = module.parse_model(invalidModel);
            console.log('❌ parse_model should have failed but returned:', invalidModelJsonString);
        } catch (error) {
            console.log('✅ parse_model correctly failed with error:', error.message);
        }
        
        // Test with empty model
        const emptyModel = `model Empty:
`;
        
        console.log('\n📝 Empty test model:', emptyModel);
        
        // Test parsing
        console.log('\n🧪 Testing parse_model with empty model...');
        const emptyModelJsonString = module.parse_model(emptyModel);
        console.log('✅ parse_model returned string:', emptyModelJsonString);
        
        // Test JSON parsing
        console.log('\n🧪 Testing JSON.parse with empty model...');
        const emptyModelJson = JSON.parse(emptyModelJsonString);
        console.log('✅ JSON.parse successful!');
        console.log('📊 Parsed empty model structure:', JSON.stringify(emptyModelJson, null, 2));
        
        // Test with just model declaration
        const justModel = `model JustModel:
`;
        
        console.log('\n📝 Just model declaration:', justModel);
        
        // Test parsing
        console.log('\n🧪 Testing parse_model with just model...');
        const justModelJsonString = module.parse_model(justModel);
        console.log('✅ parse_model returned string:', justModelJsonString);
        
        // Test JSON parsing
        console.log('\n🧪 Testing JSON.parse with just model...');
        const justModelJson = JSON.parse(justModelJsonString);
        console.log('✅ JSON.parse successful!');
        console.log('📊 Parsed just model structure:', JSON.stringify(justModelJson, null, 2));
        
        console.log('\n🎉 Parser error test completed!');
        
    } catch (error) {
        console.error('❌ Test failed:', error);
        console.error('📄 Error details:', error.message);
        console.error('📁 Stack trace:', error.stack);
    }
}

testParserError(); 