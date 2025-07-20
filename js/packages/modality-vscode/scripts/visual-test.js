#!/usr/bin/env node

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const testFile = path.join(__dirname, '../test-syntax.modality');

console.log('🎨 Visual Syntax Highlighting Test');
console.log('==================================\n');

console.log('📁 Test file location:', testFile);

// Check if test file exists
if (!fs.existsSync(testFile)) {
  console.error('❌ Test file not found:', testFile);
  process.exit(1);
}

console.log('✅ Test file found');

// Try to open the file in the default editor
try {
  console.log('\n🚀 Opening test file in default editor...');
  execSync(`open "${testFile}"`, { stdio: 'inherit' });
  console.log('✅ File opened successfully!');
} catch (error) {
  console.log('⚠️  Could not open file automatically. Please open manually:');
  console.log(`   ${testFile}`);
}

console.log('\n📋 Instructions for testing:');
console.log('1. Make sure the Modality extension is installed and active');
console.log('2. Select "Modality Dark" or "Modality Light" theme');
console.log('3. Check that diamond brackets < > are both blue');
console.log('4. Verify comparison operators <= >= are NOT blue');
console.log('5. Check that box brackets [ ] are orange');

console.log('\n🔍 What to look for:');
console.log('✅ <+red> and <-inactive> - both brackets should be blue');
console.log('✅ x <= 5 and y >= 10 - should NOT be highlighted as modal');
console.log('✅ [+always] and [-never] - brackets should be orange');
console.log('✅ Mixed contexts like "<+active> and x <= 5" - modal parts blue, comparison not');

console.log('\n💡 If colors are wrong:');
console.log('1. Restart your editor');
console.log('2. Make sure the extension is active');
console.log('3. Check the theme selection');
console.log('4. Run: pnpm run install:local'); 