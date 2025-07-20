#!/usr/bin/env node

const path = require('path');

console.log('🔍 Debugging Parser Issues');
console.log('===========================\n');

// Debug the parser issues
async function debugParser() {
    try {
        console.log('🚀 Loading WASM module...');
        
        // Load the WASM module
        const modulePath = path.resolve(__dirname, '../out/modality_lang.js');
        const modalityLang = await import(modulePath);
        const module = modalityLang.default || modalityLang;
        
        console.log('✅ Module loaded successfully!');
        
        // Test with a very simple model
        const simpleModel = `model Simple:
  part p1:
    n1 --> n2: +a
`;
        
        console.log('📝 Simple test model:', simpleModel);
        
        // Test parsing
        console.log('\n🧪 Testing parse_model...');
        const modelJsonString = module.parse_model(simpleModel);
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
        } else if (modelJson.parts && Array.isArray(modelJson.parts)) {
            console.log('⚠️  Found parts array instead of parts array');
            console.log('🔍 This suggests the parser is using old terminology');
        } else {
            console.log('❌ Neither parts nor parts array found');
            console.log('🔍 Available keys:', Object.keys(modelJson));
        }
        
        // Test with a model that should definitely work
        const workingModel = `model Working:
  part g1:
    n1 --> n2: +blue
  part g2:
    n3 --> n4: +red
`;
        
        console.log('\n📝 Working test model:', workingModel);
        
        // Test parsing
        console.log('\n🧪 Testing parse_model with working model...');
        const workingModelJsonString = module.parse_model(workingModel);
        console.log('✅ parse_model returned string:', workingModelJsonString);
        
        // Test JSON parsing
        console.log('\n🧪 Testing JSON.parse with working model...');
        const workingModelJson = JSON.parse(workingModelJsonString);
        console.log('✅ JSON.parse successful!');
        console.log('📊 Parsed working model structure:', JSON.stringify(workingModelJson, null, 2));
        
        console.log('\n🎉 Parser debug completed!');
        
    } catch (error) {
        console.error('❌ Debug failed:', error);
        console.error('📄 Error details:', error.message);
        console.error('📁 Stack trace:', error.stack);
    }
}

debugParser(); 