#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Test cases for diamond bracket highlighting
const testCases = [
  // Modal operators (should be highlighted)
  { input: '<+red>', expected: 'modal', description: 'Opening diamond with positive property' },
  { input: '<-inactive>', expected: 'modal', description: 'Opening diamond with negative property' },
  { input: '>', expected: 'modal', description: 'Closing diamond' },
  
  // Comparison operators (should NOT be highlighted as modal)
  { input: '<=', expected: 'comparison', description: 'Less than or equal' },
  { input: '>=', expected: 'comparison', description: 'Greater than or equal' },
  
  // Mixed contexts
  { input: '<+active> and x <= 5', expected: 'mixed', description: 'Modal and comparison mixed' },
  
  // Box operators
  { input: '[+always]', expected: 'box', description: 'Box operator' },
  { input: '[-never]', expected: 'box', description: 'Box operator negative' }
];

// Simple regex patterns to test
const patterns = {
  modalOpen: /<(?!\=)/,
  modalClose: />(?!\=)/,
  comparison: /(<=|>=)/,
  boxOpen: /\[/,
  boxClose: /\]/
};

function testPattern(text, pattern, name) {
  const matches = text.match(pattern);
  return {
    pattern: name,
    matches: matches ? matches.length : 0,
    found: matches || []
  };
}

function runTests() {
  console.log('🧪 Testing Modality Syntax Highlighting Patterns\n');
  
  let passed = 0;
  let total = 0;
  
  testCases.forEach((testCase, index) => {
    console.log(`Test ${index + 1}: ${testCase.description}`);
    console.log(`Input: "${testCase.input}"`);
    
    const results = {
      modalOpen: testPattern(testCase.input, patterns.modalOpen, 'Modal Open'),
      modalClose: testPattern(testCase.input, patterns.modalClose, 'Modal Close'),
      comparison: testPattern(testCase.input, patterns.comparison, 'Comparison'),
      boxOpen: testPattern(testCase.input, patterns.boxOpen, 'Box Open'),
      boxClose: testPattern(testCase.input, patterns.boxClose, 'Box Close')
    };
    
    let testPassed = false;
    
    switch (testCase.expected) {
      case 'modal':
        testPassed = results.modalOpen.matches > 0 || results.modalClose.matches > 0;
        break;
      case 'comparison':
        testPassed = results.comparison.matches > 0;
        break;
      case 'mixed':
        testPassed = (results.modalOpen.matches > 0 || results.modalClose.matches > 0) && 
                    results.comparison.matches > 0;
        break;
      case 'box':
        testPassed = results.boxOpen.matches > 0 || results.boxClose.matches > 0;
        break;
    }
    
    if (testPassed) {
      console.log('✅ PASSED');
      passed++;
    } else {
      console.log('❌ FAILED');
      console.log('Results:', results);
    }
    
    console.log('');
    total++;
  });
  
  console.log(`\n📊 Results: ${passed}/${total} tests passed`);
  
  if (passed === total) {
    console.log('🎉 All tests passed! Syntax highlighting patterns are working correctly.');
  } else {
    console.log('⚠️  Some tests failed. Check the patterns above.');
  }
}

// Check if syntax file exists
const syntaxFile = path.join(__dirname, '../syntaxes/modality.tmLanguage.json');
if (!fs.existsSync(syntaxFile)) {
  console.error('❌ Syntax file not found:', syntaxFile);
  process.exit(1);
}

console.log('📁 Found syntax file:', syntaxFile);
runTests(); 