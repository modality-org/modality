#!/usr/bin/env node

const path = require('path');

console.log('🧪 Testing Complex Model Parsing');
console.log('================================\n');

// Test the complex model parsing
async function testComplexModel() {
    try {
        console.log('🚀 Loading WASM module...');
        
        // Load the WASM module
        const modulePath = path.resolve(__dirname, '../out/modality_lang.js');
        const modalityLang = await import(modulePath);
        const module = modalityLang.default || modalityLang;
        
        console.log('✅ Module loaded successfully!');
        
        // Test complex model with multiple parts
        const testModel = `model TestModel:
  part p1:
    n1 --> n2: +a
    n2 --> n3: +b
    n3 --> n1: -c
  
  part p2:
    n4 --> n5: +d
    n5 --> n6: +e
    n6 --> n4: -f
  
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
        console.log('📊 Parsed model structure:', JSON.stringify(modelJson, null, 2));
        
        // Check if parts are correctly parsed
        if (modelJson.parts && Array.isArray(modelJson.parts)) {
            console.log('✅ Parts array found with', modelJson.parts.length, 'parts');
            modelJson.parts.forEach((part, index) => {
                console.log(`  📦 Part ${index + 1}: ${part.name} with ${part.transitions.length} transitions`);
            });
        } else {
            console.log('❌ Parts array not found or not an array');
            console.log('🔍 Available keys:', Object.keys(modelJson));
        }
        
        // Test Mermaid generation
        console.log('\n🧪 Testing generate_mermaid_styled...');
        const mermaidContent = module.generate_mermaid_styled(modelJsonString);
        console.log('✅ generate_mermaid_styled successful!');
        console.log('📊 Mermaid content:');
        console.log(mermaidContent);
        
        console.log('\n🎉 Complex model test completed successfully!');
        
    } catch (error) {
        console.error('❌ Test failed:', error);
        console.error('📄 Error details:', error.message);
        console.error('📁 Stack trace:', error.stack);
    }
}

testComplexModel(); 