// Use the Modality language parser and model checker from Node.js.
//
// Build the bindings first, from rust/modality-lang:
//   wasm-pack build --target nodejs --out-dir dist-node
// then run: node examples/wasm/node-example.cjs

const modalityLang = require('../../dist-node/modality_lang.js');

const modalityCode = `model TestModel {
  part g1 {
    n1 --> n2: +blue -red
    n2 --> n3: +green
    n3 --> n1: -blue +yellow
  }
  part g2 {
    a --> b: +init
    b --> c: +complete
    c --> a: +reset
  }
}
`;

const formulaCode = `formula CanGoBlue {
  <+blue> true
}
`;

function section(title) {
    console.log('\n' + '='.repeat(50) + '\n');
    console.log(title);
}

function main() {
    console.log('📝 Example Modality code:');
    console.log(modalityCode);

    // The parse functions return plain objects; the diagram and checker
    // functions take them as JSON text.
    section('🔍 Parsing a single model...');
    const model = modalityLang.parse_model(modalityCode);
    console.log(`✅ ${model.name}: ${model.parts.length} parts`);
    const modelJson = JSON.stringify(model);

    section('🔍 Parsing all models...');
    const models = modalityLang.parse_all_models(modalityCode);
    console.log(`✅ ${models.length} model(s): ${models.map((m) => m.name).join(', ')}`);

    section('📊 Mermaid diagram');
    console.log('```mermaid');
    console.log(modalityLang.generate_mermaid(modelJson));
    console.log('```');

    section('🎨 Styled Mermaid diagram');
    console.log('```mermaid');
    console.log(modalityLang.generate_mermaid_styled(modelJson));
    console.log('```');

    // A model file does not say where a run is. Set the current nodes on the
    // parsed model; several in one part is non-determinism.
    section('🎯 State-aware Mermaid diagram (g1 at n1 or n2, g2 at a)');
    const withState = {
        ...model,
        state: [
            { part_name: 'g1', current_nodes: ['n1', 'n2'] },
            { part_name: 'g2', current_nodes: ['a'] },
        ],
    };
    console.log('```mermaid');
    console.log(modalityLang.generate_mermaid_with_state(JSON.stringify(withState)));
    console.log('```');

    section('✔️  Checking a formula');
    const [formula] = modalityLang.parse_formulas(formulaCode);
    const result = modalityLang.check_formula_any_state(modelJson, JSON.stringify(formula));
    console.log(`${formula.name}: ${result.is_satisfied ? 'satisfied' : 'not satisfied'}`);

    section('🏗️  The ModalityParser class');
    const parser = new modalityLang.ModalityParser();
    const parsed = parser.parse_model(modalityCode);
    console.log(`✅ ${parsed.name}: ${parsed.parts.length} parts`);
}

main();
