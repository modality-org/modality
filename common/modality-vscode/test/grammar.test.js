// Tokenizes .modality files with the engine VS Code uses and checks scopes.
//
//   npm run test:grammar
//
// 1. Scope checks on test/fixtures/all-syntax.modality, and recovery
//    checks: half-typed text leaves the next declaration highlighted.
// 2. Every .modality file in the repo ends with no block left open, so a
//    brace the grammar mishandles shows up as a failure, not as a file that
//    is highlighted wrong from that line on.

const fs = require('fs');
const path = require('path');
const vsctm = require('vscode-textmate');
const oniguruma = require('vscode-oniguruma');

const root = path.join(__dirname, '..');
const repo = path.join(root, '..', '..');

async function loadGrammar() {
    const wasm = fs.readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm'));
    await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
    const registry = new vsctm.Registry({
        onigLib: Promise.resolve({
            createOnigScanner: (sources) => new oniguruma.OnigScanner(sources),
            createOnigString: (s) => new oniguruma.OnigString(s),
        }),
        loadGrammar: async () => {
            const file = path.join(root, 'syntaxes', 'modality.tmLanguage.json');
            return vsctm.parseRawGrammar(fs.readFileSync(file, 'utf8'), file);
        },
    });
    return registry.loadGrammar('source.modality');
}

function tokenize(grammar, text) {
    let stack = vsctm.INITIAL;
    const lines = text.split(/\r?\n/).map((line) => {
        const r = grammar.tokenizeLine(line, stack);
        stack = r.ruleStack;
        return r.tokens.map((t) => ({
            text: line.slice(t.startIndex, t.endIndex),
            scopes: t.scopes,
        }));
    });
    return { lines, depth: stack.depth };
}

// [line containing, token text, scope prefix expected on that token]
const EXPECT = [
    ['block', 'block', 'comment.block'],
    ['model Escrow', 'model', 'storage.type.model'],
    ['model Escrow', 'Escrow', 'entity.name.type.model'],
    ['initial idle', 'initial', 'keyword.other.initial'],
    ['initial idle', 'idle', 'entity.name.tag.state'],
    ['idle -> funded', '->', 'keyword.operator.arrow'],
    ['idle -> funded', 'funded', 'entity.name.tag.state'],
    ['idle -> funded', 'DEPOSIT', 'constant.other.action'],
    ['idle -> funded', 'signed_by', 'entity.name.function.predicate'],
    ['idle -> funded', '/parties/buyer.id', 'string.unquoted.path'],
    ['funded --> done', '-->', 'keyword.operator.arrow'],
    ['funded --> done', 'RELEASE', 'constant.other.action'],
    ['funded --> done', 'signed_by', 'entity.name.function.predicate'],
    ['funded --> done', '/parties/arbiter.id', 'string.unquoted.path'],
    ['funded --> done', 'hello', 'constant.other.property'],
    ['part flow', 'part', 'storage.type.part'],
    ['part flow', 'flow', 'entity.name.type.part'],
    ['a -> b [-bool_true', 'bool_true', 'entity.name.function.predicate'],
    ['a -> b [-bool_true', 'lp', 'string.quoted.double'],
    ['a -> b [-bool_true', '3', 'constant.numeric'],
    ['export default model', 'export', 'keyword.control.export'],
    ['export default model', 'model', 'storage.type.model'],
    ['formula Committed', 'Committed', 'entity.name.function.formula'],
    ['[<+signed_by', '[<', 'keyword.operator.modal.diamond-box'],
    ['[<+signed_by', 'signed_by', 'entity.name.function.predicate'],
    ['[<+signed_by', '>]', 'keyword.operator.modal.diamond-box'],
    ['[<+signed_by', 'true', 'constant.language.boolean'],
    ['[<+signed_by', 'goal', 'variable.other.proposition'],
    ['[<+signed_by', '->', 'keyword.operator.logical'],
    ['lfp(X', 'lfp', 'keyword.control.fixedpoint'],
    ['lfp(X', 'X', 'variable.parameter.fixedpoint'],
    ['lfp(X', 'and', 'keyword.operator.logical'],
    ['lfp(X', 'μ', 'keyword.control.fixedpoint'],
    ['lfp(X', 'implies', 'keyword.operator.logical'],
    ['always([+modifies', 'always', 'keyword.control.temporal'],
    ['always([+modifies', 'modifies', 'entity.name.function.predicate'],
    ['always([+modifies', 'all_signed', 'entity.name.function.predicate'],
    ['always([+modifies', '/maintainers', 'string.unquoted.path'],
    ['always([+modifies', 'false', 'constant.language.boolean'],
    ['always([+modifies', 'until', 'keyword.control.temporal'],
    ['always([+modifies', '□', 'keyword.control.temporal'],
    ['export default rule', 'rule', 'storage.type.rule'],
    ['starting_at', 'starting_at', 'keyword.other.starting-at'],
    ['starting_at', '$PARENT', 'variable.language.anchor'],
    ['always([+CREATE]', 'CREATE', 'constant.other.action'],
    ['rule named', 'named', 'entity.name.function.rule'],
    ['formula inner', 'formula', 'storage.type.formula'],
    ['formula inner', 'inner', 'entity.name.function.formula'],
    ['action Pay', 'Pay', 'entity.name.function.action'],
    ['+PAY -signed_by', 'PAY', 'constant.other.action'],
    ['test check', 'check', 'entity.name.function.test'],
    ['clone(Escrow)', 'clone', 'support.function.test'],
    ['m.commit(Pay)', 'commit', 'support.function.test'],
    ['assert m', 'assert', 'keyword.control.assert'],
    ['assert m', 'satisfies', 'support.function.test'],
    ['contract deal', 'deal', 'entity.name.type.contract'],
    ['commit {', 'commit', 'storage.type.commit'],
    ['signed_by alice', 'signed_by', 'keyword.other.signed-by'],
    ['signed_by alice', 'sig', 'string.quoted.double'],
    ['add_rule', 'add_rule', 'keyword.other.add-rule'],
    ['add_rule', 'STOP', 'constant.other.action'],
    ['add_rule', 'always', 'keyword.control.temporal'],
    ['do +GO', 'do', 'keyword.other.do'],
    ['do +GO', 'GO', 'constant.other.action'],
    ['rule_for_this_commit', 'rule_for_this_commit', 'storage.type.rule'],
    ['signed_by_n', 'signed_by_n', 'entity.name.function.predicate'],
    ['signed_by_n', '2', 'constant.numeric'],
    ['signed_by_n', '/a.id', 'string.unquoted.path'],
    ['always([-signed_by(/users/alice.id)] false)', 'always', 'keyword.control.temporal'],
    ['always([-signed_by(/users/alice.id)] false)', 'signed_by', 'entity.name.function.predicate'],
    ['always([-signed_by(/users/alice.id)] false)', '/users/alice.id', 'string.unquoted.path'],
    ['<+PAY> true', 'PAY', 'constant.other.action'],
    ['<+PAY> true', 'true', 'constant.language.boolean'],
    ['+threshold("2"', 'threshold', 'entity.name.function.predicate'],
    ['+threshold("2"', '/signers', 'string.unquoted.path'],
    ['q0 -> q1 [+ACTION_NAME]', 'q0', 'entity.name.tag.state'],
    ['q0 -> q1 [+ACTION_NAME]', 'ACTION_NAME', 'constant.other.action'],
    ['initial start', 'initial', 'keyword.other.initial'],
    ['initial start', 'start', 'entity.name.tag.state'],
    ['bool_true(/status', 'bool_true', 'entity.name.function.predicate'],
    ['bool_true(/status', '/status/delivered.bool', 'string.unquoted.path'],
    ['!+modifies(/members)', 'modifies', 'entity.name.function.predicate'],
    ['!+modifies(/members)', 'all_signed', 'entity.name.function.predicate'],
];

function checkFixture(grammar) {
    const file = path.join(__dirname, 'fixtures', 'all-syntax.modality');
    const text = fs.readFileSync(file, 'utf8');
    const src = text.split(/\r?\n/);
    const { lines, depth } = tokenize(grammar, text);
    const failures = [];
    if (depth !== 1) failures.push(`fixture ends ${depth - 1} block(s) deep`);
    for (const [where, tokenText, scope] of EXPECT) {
        const n = src.findIndex((l) => l.includes(where));
        if (n < 0) {
            failures.push(`no line contains ${JSON.stringify(where)}`);
            continue;
        }
        const tok = lines[n].find((t) => t.text.trim() === tokenText);
        if (!tok) {
            const got = lines[n].map((t) => t.text.trim()).filter(Boolean);
            failures.push(`line ${n + 1}: no token ${JSON.stringify(tokenText)} (tokens: ${got.join(' | ')})`);
            continue;
        }
        if (!tok.scopes.some((s) => s.startsWith(scope))) {
            failures.push(`line ${n + 1}: ${JSON.stringify(tokenText)} is ${tok.scopes.slice(1).join(' ')}, want ${scope}`);
        }
    }
    return failures;
}

// Half-typed or old-syntax text must not swallow the declarations after it.
const RECOVERY = [
    'formula F {\n  always([+modifies(/keys\n}\n',
    'formula F {\n  always([<+A> true]\n}\n',
    'model Old:\n  part g:\n    a --> b: +x\n',
    'formula F {\n  <+signed_by(\n}\n',
];

function checkRecovery(grammar) {
    const failures = [];
    for (const head of RECOVERY) {
        const { lines, depth } = tokenize(grammar, `${head}model After {\n  a -> b\n}\n`);
        const after = lines.find((l) => l.some((t) => t.text === 'After'));
        const name = after && after.find((t) => t.text === 'After');
        if (depth !== 1 || !name || !name.scopes.some((s) => s.startsWith('entity.name.type.model'))) {
            failures.push(`recovery after ${JSON.stringify(head)}`);
        }
    }
    return failures;
}

function modalityFiles(dir, out = []) {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
        if (['node_modules', 'target', '.git'].includes(e.name)) continue;
        const p = path.join(dir, e.name);
        if (e.isDirectory()) modalityFiles(p, out);
        else if (e.name.endsWith('.modality')) out.push(p);
    }
    return out;
}

function checkRepo(grammar) {
    const files = modalityFiles(repo);
    const failures = [];
    for (const f of files) {
        const { depth } = tokenize(grammar, fs.readFileSync(f, 'utf8'));
        if (depth !== 1) failures.push(`${path.relative(repo, f)}: ends ${depth - 1} block(s) deep`);
    }
    return { count: files.length, failures };
}

(async () => {
    const grammar = await loadGrammar();
    const fixture = [...checkFixture(grammar), ...checkRecovery(grammar)];
    const repoCheck = checkRepo(grammar);
    for (const f of [...fixture, ...repoCheck.failures]) console.log(`FAIL ${f}`);
    console.log(`${EXPECT.length} scope checks, ${RECOVERY.length} recovery checks, ${repoCheck.count} repo files`);
    const n = fixture.length + repoCheck.failures.length;
    if (n) {
        console.log(`${n} failure(s)`);
        process.exit(1);
    }
    console.log('ok');
})();
