#!/usr/bin/env node

const path = require('path');

console.log('🧪 Testing JSON Parsing Fix');
console.log('============================\n');

// Test the JSON parsing fix
async function testJsonParsing() {
    try {
        console.log('🚀 Loading WASM module...');
        
        // Load the WASM module
        const modulePath = path.resolve(__dirname, '../out/modality_lang.js');
        const modalityLang = await import(modulePath);
        const module = modalityLang.default || modalityLang;
        
        console.log('✅ Module loaded successfully!');
        
        // Test model with formulas
        const testModel = `model TestModel:
  part p1:
    n1 --> n2: +a
    n2 --> n3: +b
  
  part p2:
    n4 --> n5: +c
  
  formula f1: <p1> true
  formula f2: [p2] false
`;
        
        console.log('📝 Test model:', testModel);
        
        // Test parsing
        console.log('\n🧪 Testing parse_model...');
        const modelJsonString = module.parse_model(testModel);
        console.log('✅ parse_model returned string:', modelJsonString);
        
        // Test JSON parsing
        console.log('\n🧪 Testing JSON.parse...');
        const modelJson = JSON.parse(modelJsonString);
        console.log('✅ JSON.parse successful!');
        console.log('📊 Parsed model:', JSON.stringify(modelJson, null, 2));
        
        // Test Mermaid generation
        console.log('\n🧪 Testing generate_mermaid_styled...');
        const mermaidContent = module.generate_mermaid_styled(modelJsonString);
        console.log('✅ generate_mermaid_styled successful!');
        console.log('📊 Mermaid content:', mermaidContent);
        
        console.log('\n🎉 JSON parsing test completed successfully!');
        
    } catch (error) {
        console.error('❌ Test failed:', error);
        console.error('📄 Error details:', error.message);
        console.error('📁 Stack trace:', error.stack);
    }
}

testJsonParsing(); 