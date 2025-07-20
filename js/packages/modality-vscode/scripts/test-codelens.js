#!/usr/bin/env node

console.log('🔍 Testing Modality CodeLens Feature');
console.log('====================================\n');

// Sample model with multiple model declarations
const sampleContent = `model SimpleModel:

part StateMachine:
    idle --> active: +start
    active --> processing: +request
    processing --> active: +response

model ComplexModel:

part Controller:
    init --> running: +boot
    running --> paused: +pause
    paused --> running: +resume

model TestModel:
    // This model has no parts
`;

console.log('📋 Sample Modality file with multiple models:');
console.log('============================================');
console.log(sampleContent);

console.log('\n🎯 CodeLens Detection:');
console.log('=====================');

// Simulate CodeLens detection
const lines = sampleContent.split('\n');
let modelCount = 0;

for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmedLine = line.trim();
    
    // Check for model declaration
    const modelMatch = trimmedLine.match(/^model\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*:/);
    if (modelMatch) {
        const modelName = modelMatch[1];
        modelCount++;
        console.log(`✅ Line ${i + 1}: Found model "${modelName}"`);
        console.log(`   📍 Range: Line ${i + 1}, Column 0 to ${line.length}`);
        console.log(`   🎯 CodeLens: "Visualize" button will appear here`);
        console.log('');
    }
}

console.log(`📊 Summary: Found ${modelCount} model declaration(s)`);
console.log('\n💡 How to test in VS Code/Cursor:');
console.log('1. Open a .modality file with model declarations');
console.log('2. Look for "Visualize" buttons above each model line');
console.log('3. Click the button to generate Mermaid diagram');
console.log('4. The diagram will open in a new tab');

console.log('\n🎉 CodeLens test completed!'); 