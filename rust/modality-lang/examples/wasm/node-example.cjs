const { readFileSync } = require('fs');
const path = require('path');

// Import the WASM module
const modalityLang = require('./modality_lang.js');

async function main() {
    try {
        // WASM module is automatically initialized in Node.js version
        console.log('✅ WASM module is ready to use!\n');

        // Example Modality language code
        const modalityCode = `model TestModel:
  graph g1:
    n1 --> n2 : +blue -red
    n2 --> n3 : +green
    n3 --> n1 : -blue +yellow
  graph g2:
    a --> b : +init
    b --> c : +complete
    c --> a : +reset`;

        console.log('📝 Example Modality Code:');
        console.log(modalityCode);
        console.log('\n' + '='.repeat(50) + '\n');

        // Parse a single model
        console.log('🔍 Parsing single model...');
        try {
            const modelResult = modalityLang.parse_model(modalityCode);
            const model = JSON.parse(modelResult);
            console.log('✅ Single model parsed successfully!');
            console.log('Model structure:', JSON.stringify(model, null, 2));
        } catch (error) {
            console.error('❌ Error parsing single model:', error.message);
        }
        console.log('\n' + '='.repeat(50) + '\n');

        // Parse all models
        console.log('🔍 Parsing all models...');
        try {
            const modelsResult = modalityLang.parse_all_models(modalityCode);
            const models = JSON.parse(modelsResult);
            console.log('✅ All models parsed successfully!');
            console.log('Models structure:', JSON.stringify(models, null, 2));
        } catch (error) {
            console.error('❌ Error parsing all models:', error.message);
        }
        console.log('\n' + '='.repeat(50) + '\n');

        // Generate Mermaid diagram for single model
        console.log('📊 Generating Mermaid diagram for single model...');
        try {
            const modelResult = modalityLang.parse_model(modalityCode);
            const mermaidResult = modalityLang.generate_mermaid(modelResult);
            console.log('✅ Mermaid diagram generated successfully!');
            console.log('Mermaid diagram:');
            console.log('```mermaid');
            console.log(mermaidResult);
            console.log('```');
        } catch (error) {
            console.error('❌ Error generating Mermaid diagram:', error.message);
        }
        console.log('\n' + '='.repeat(50) + '\n');

        // Generate styled Mermaid diagram
        console.log('🎨 Generating styled Mermaid diagram...');
        try {
            const modelResult = modalityLang.parse_model(modalityCode);
            const styledMermaidResult = modalityLang.generate_mermaid_styled(modelResult);
            console.log('✅ Styled Mermaid diagram generated successfully!');
            console.log('Styled Mermaid diagram:');
            console.log('```mermaid');
            console.log(styledMermaidResult);
            console.log('```');
        } catch (error) {
            console.error('❌ Error generating styled Mermaid diagram:', error.message);
        }

        // Test the ModalityParser class
        console.log('\n' + '='.repeat(50) + '\n');
        console.log('🏗️  Testing ModalityParser class...');
        try {
            const parser = new modalityLang.ModalityParser();
            const modelResult = parser.parse_model(modalityCode);
            const model = JSON.parse(modelResult);
            console.log('✅ ModalityParser class works!');
            console.log('Model name:', model.name);
            console.log('Number of graphs:', model.graphs.length);
        } catch (error) {
            console.error('❌ Error with ModalityParser class:', error.message);
        }

    } catch (error) {
        console.error('💥 Fatal error:', error);
        process.exit(1);
    }
}

// Run the example
main(); 