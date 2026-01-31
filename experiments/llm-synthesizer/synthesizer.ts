#!/usr/bin/env npx ts-node
/**
 * LLM-Enhanced Modality Synthesizer
 * 
 * Takes natural language contract descriptions and generates:
 * 1. Modality models (state machines)
 * 2. Temporal logic formulas (rules)
 * 3. Protection guarantees for each party
 */

import Anthropic from "@anthropic-ai/sdk";
import * as fs from "fs";
import * as path from "path";
import { execSync, spawn } from "child_process";

const client = new Anthropic();

// System prompt with Modality syntax and examples
const SYSTEM_PROMPT = `You are a formal verification expert specializing in Modality, a verification language for AI agent cooperation.

## Modality Overview

Modality enables agents to negotiate verifiable contracts. A contract consists of:
1. **Model**: A state machine (Labeled Transition System) describing possible states and transitions
2. **Rules**: Temporal logic formulas that constrain valid transitions
3. **Predicates**: WASM modules for signature verification

## Model Syntax

\`\`\`modality
model ContractName {
  part flow {
    state1 --> state2: +REQUIRED_ACTION -FORBIDDEN_ACTION
    state2 --> state2: +ACTION1 +ACTION2
    state2 --> final
    final --> final
  }
}
\`\`\`

- \`+ACTION\` — transition REQUIRES this action
- \`-ACTION\` — transition FORBIDS this action
- No mention — transition is NEUTRAL (action can be present or absent)
- States are lowercase, actions are UPPERCASE

## Formula Syntax (Temporal Modal Logic)

### Modal Operators
- \`[+A] φ\` — ALL +A transitions lead to φ (box)
- \`<+A> φ\` — EXISTS a +A transition to φ (diamond)
- \`[<+A>] φ\` — COMMITTED to A: can do +A AND cannot refuse (diamondbox)
- \`[] φ\` — ALL transitions lead to φ
- \`<> φ\` — SOME transition leads to φ

### Temporal Operators
- \`always(φ)\` — φ holds forever on all paths
- \`eventually(φ)\` — φ holds at some future state
- \`until(p, q)\` — p holds until q becomes true

### Fixed Points (advanced)
- \`gfp(X, φ)\` — greatest fixed point (invariants)
- \`lfp(X, φ)\` — least fixed point (reachability)

## Rule File Structure

\`\`\`modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [<+signed_by(/users/alice.id)>] true | [<+signed_by(/users/bob.id)>] true
    )
  }
}
\`\`\`

## Common Patterns

### Escrow
- Buyer deposits → Seller delivers → Buyer releases payment
- Protection: Buyer can't lose funds without delivery; Seller gets paid after delivery

### Handshake
- Both parties must sign before contract activates
- Protection: Neither party is bound until both agree

### Atomic Swap
- Both parties commit before either can claim
- Protection: Neither party loses value without receiving the other's

### Service Agreement
- Provider offers → Consumer accepts → Provider delivers → Consumer confirms → Payment
- Protection: Both parties must fulfill obligations

### Delegation
- Principal grants agent authority to act on their behalf
- Protection: Principal can revoke; Agent actions are bounded

## Your Task

Given a natural language description of a contract:

1. **Analyze** the parties, their roles, and trust requirements
2. **Generate** a Modality model with appropriate states and transitions
3. **Generate** rules that protect each party
4. **Explain** the protections each party receives

Output format:
\`\`\`
## Analysis
[Party identification and trust requirements]

## Model
\`\`\`modality
model ContractName {
  ...
}
\`\`\`

## Rules

### Rule: [description]
\`\`\`modality
export default rule {
  ...
}
\`\`\`

## Protections
- **Party A**: [what they're protected against]
- **Party B**: [what they're protected against]
\`\`\`

Be precise with syntax. Use lowercase for state names, UPPERCASE for actions.
Use +signed_by(/path) for signature requirements.`;

interface SynthesisResult {
  analysis: string;
  model: string;
  rules: string[];
  protections: Record<string, string>;
  raw: string;
}

interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

async function synthesize(description: string): Promise<SynthesisResult> {
  console.log("🔄 Synthesizing contract from description...\n");
  
  const response = await client.messages.create({
    model: "claude-sonnet-4-20250514",
    max_tokens: 4096,
    system: SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: `Please synthesize a Modality contract for the following scenario:

${description}

Generate a complete model with appropriate states and transitions, plus rules that protect each party.`
      }
    ]
  });

  const content = response.content[0];
  if (content.type !== "text") {
    throw new Error("Unexpected response type");
  }

  const raw = content.text;
  
  // Parse the response
  const result = parseResponse(raw);
  
  return result;
}

function parseResponse(raw: string): SynthesisResult {
  // Extract sections
  const analysisMatch = raw.match(/## Analysis\s*\n([\s\S]*?)(?=##|```modality|$)/);
  const modelMatch = raw.match(/```modality\s*\nmodel\s+(\w+)\s*\{([\s\S]*?)\}\s*```/);
  const ruleMatches = [...raw.matchAll(/```modality\s*\nexport default rule\s*\{([\s\S]*?)\}\s*```/g)];
  const protectionsMatch = raw.match(/## Protections\s*\n([\s\S]*?)(?=##|$)/);

  const analysis = analysisMatch ? analysisMatch[1].trim() : "";
  
  let model = "";
  if (modelMatch) {
    model = `model ${modelMatch[1]} {${modelMatch[2]}}`;
  }

  const rules = ruleMatches.map(m => `export default rule {${m[1]}}`);

  const protections: Record<string, string> = {};
  if (protectionsMatch) {
    const lines = protectionsMatch[1].split("\n");
    for (const line of lines) {
      const match = line.match(/\*\*([^*]+)\*\*:\s*(.+)/);
      if (match) {
        protections[match[1].trim()] = match[2].trim();
      }
    }
  }

  return { analysis, model, rules, protections, raw };
}

async function validate(model: string): Promise<ValidationResult> {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Basic syntax checks
  if (!model.includes("model ")) {
    errors.push("Missing model declaration");
  }

  if (!model.includes("part ")) {
    errors.push("Missing part declaration");
  }

  if (!model.includes("-->")) {
    errors.push("No transitions found");
  }

  // Check for common issues
  if (model.match(/[A-Z][a-z]+\s*-->/)) {
    warnings.push("State names should be lowercase");
  }

  if (model.match(/\+[a-z_]+[^A-Z]/)) {
    warnings.push("Action names should be UPPERCASE");
  }

  // Try to parse with modality CLI if available
  try {
    const tempFile = `/tmp/synthesized_${Date.now()}.modality`;
    fs.writeFileSync(tempFile, model);
    
    const result = execSync(`cd /root/.openclaw/workspace/modality/rust && cargo run --bin modality -- parse ${tempFile} 2>&1`, {
      encoding: "utf-8",
      timeout: 30000
    });
    
    if (result.includes("error") || result.includes("Error")) {
      errors.push(result);
    }
    
    fs.unlinkSync(tempFile);
  } catch (e: any) {
    // Parser not available or error
    if (e.message && e.message.includes("error")) {
      errors.push(e.message);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings
  };
}

async function refine(
  description: string,
  previousResult: SynthesisResult,
  validationResult: ValidationResult
): Promise<SynthesisResult> {
  console.log("🔧 Refining based on validation errors...\n");

  const response = await client.messages.create({
    model: "claude-sonnet-4-20250514",
    max_tokens: 4096,
    system: SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: `Please synthesize a Modality contract for the following scenario:

${description}`
      },
      {
        role: "assistant",
        content: previousResult.raw
      },
      {
        role: "user",
        content: `The generated model has validation issues:

Errors:
${validationResult.errors.map(e => `- ${e}`).join("\n")}

Warnings:
${validationResult.warnings.map(w => `- ${w}`).join("\n")}

Please fix these issues and regenerate the model. Remember:
- State names should be lowercase (e.g., pending, delivered)
- Action names should be UPPERCASE (e.g., +PAY, +DELIVER)
- Use +signed_by(/users/name.id) for signature requirements
- Every part needs at least one transition`
      }
    ]
  });

  const content = response.content[0];
  if (content.type !== "text") {
    throw new Error("Unexpected response type");
  }

  return parseResponse(content.text);
}

async function interactiveSession(): Promise<void> {
  const readline = await import("readline");
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });

  const question = (prompt: string): Promise<string> => {
    return new Promise((resolve) => {
      rl.question(prompt, resolve);
    });
  };

  console.log("\n🔐 Modality LLM Synthesizer - Interactive Mode\n");
  console.log("Describe your contract scenario in natural language.");
  console.log("Type 'quit' to exit, 'examples' for sample prompts.\n");

  const examples = [
    "Alice wants to hire Bob for a coding task. She pays 100 tokens after he delivers working code.",
    "Three agents need to approve actions with 2-of-3 multisig.",
    "A marketplace where sellers list items, buyers bid, and payment is escrowed.",
    "Agent A delegates authority to Agent B to act on their behalf, but can revoke anytime.",
    "Two AI agents want to exchange data atomically - neither loses data without getting the other's."
  ];

  while (true) {
    const input = await question("📝 Describe your contract: ");
    
    if (input.toLowerCase() === "quit") {
      console.log("\nGoodbye! 🔐");
      break;
    }

    if (input.toLowerCase() === "examples") {
      console.log("\n📚 Example prompts:\n");
      examples.forEach((ex, i) => console.log(`${i + 1}. ${ex}`));
      console.log("");
      continue;
    }

    if (!input.trim()) {
      continue;
    }

    try {
      // Initial synthesis
      let result = await synthesize(input);
      console.log("\n" + result.raw + "\n");

      // Validate
      if (result.model) {
        const validation = await validate(result.model);
        
        if (!validation.valid) {
          console.log("⚠️ Validation issues found:");
          validation.errors.forEach(e => console.log(`  ❌ ${e}`));
          validation.warnings.forEach(w => console.log(`  ⚠️ ${w}`));
          
          const retry = await question("\nAttempt to fix? (y/n): ");
          if (retry.toLowerCase() === "y") {
            result = await refine(input, result, validation);
            console.log("\n" + result.raw + "\n");
          }
        } else {
          console.log("✅ Model validated successfully!\n");
        }
      }

      // Ask for refinement
      const refinePrompt = await question("Any refinements? (or press Enter to continue): ");
      if (refinePrompt.trim()) {
        const refinedResult = await synthesize(`${input}\n\nAdditional requirements: ${refinePrompt}`);
        console.log("\n" + refinedResult.raw + "\n");
      }

    } catch (error: any) {
      console.error(`\n❌ Error: ${error.message}\n`);
    }
  }

  rl.close();
}

async function runExperiments(): Promise<void> {
  console.log("🧪 Running synthesis experiments...\n");

  const testCases = [
    {
      name: "Simple Escrow",
      description: "Alice wants to buy something from Bob. She deposits payment, Bob delivers the goods, then Alice releases the payment."
    },
    {
      name: "Task Delegation",
      description: "AgentA delegates authority to AgentB to perform tasks on their behalf. AgentA can revoke this at any time."
    },
    {
      name: "Atomic Data Exchange",
      description: "Two AI agents want to exchange datasets. Neither should receive data without the other receiving theirs too."
    },
    {
      name: "Multi-party Approval",
      description: "A DAO has 5 members. Any action requires approval from at least 3 of them."
    },
    {
      name: "Service with Milestones",
      description: "A contractor has 3 milestones: Design, Implementation, and Testing. The client pays after each milestone is delivered and approved."
    }
  ];

  const results: Array<{
    name: string;
    success: boolean;
    model?: string;
    rules?: string[];
    errors?: string[];
  }> = [];

  for (const testCase of testCases) {
    console.log(`\n${"=".repeat(60)}`);
    console.log(`📋 Test: ${testCase.name}`);
    console.log(`${"=".repeat(60)}\n`);
    console.log(`Description: ${testCase.description}\n`);

    try {
      const result = await synthesize(testCase.description);
      
      console.log("📄 Generated Model:");
      console.log(result.model || "(no model generated)");
      console.log("");

      if (result.rules.length > 0) {
        console.log("📜 Generated Rules:");
        result.rules.forEach((rule, i) => {
          console.log(`\nRule ${i + 1}:`);
          console.log(rule);
        });
      }

      if (Object.keys(result.protections).length > 0) {
        console.log("\n🛡️ Protections:");
        for (const [party, protection] of Object.entries(result.protections)) {
          console.log(`  ${party}: ${protection}`);
        }
      }

      // Validate
      if (result.model) {
        const validation = await validate(result.model);
        if (validation.valid) {
          console.log("\n✅ Validation: PASSED");
          results.push({ name: testCase.name, success: true, model: result.model, rules: result.rules });
        } else {
          console.log("\n⚠️ Validation: FAILED");
          validation.errors.forEach(e => console.log(`  ❌ ${e}`));
          
          // Try to refine
          console.log("\n🔧 Attempting refinement...");
          const refined = await refine(testCase.description, result, validation);
          
          if (refined.model) {
            const revalidation = await validate(refined.model);
            if (revalidation.valid) {
              console.log("✅ Refinement successful!");
              results.push({ name: testCase.name, success: true, model: refined.model, rules: refined.rules });
            } else {
              console.log("❌ Refinement still has issues");
              results.push({ name: testCase.name, success: false, errors: revalidation.errors });
            }
          }
        }
      }

    } catch (error: any) {
      console.error(`❌ Error: ${error.message}`);
      results.push({ name: testCase.name, success: false, errors: [error.message] });
    }
  }

  // Summary
  console.log(`\n${"=".repeat(60)}`);
  console.log("📊 EXPERIMENT SUMMARY");
  console.log(`${"=".repeat(60)}\n`);

  const passed = results.filter(r => r.success).length;
  const failed = results.filter(r => !r.success).length;

  console.log(`Total: ${results.length} | Passed: ${passed} | Failed: ${failed}\n`);

  for (const result of results) {
    const status = result.success ? "✅" : "❌";
    console.log(`${status} ${result.name}`);
    if (!result.success && result.errors) {
      result.errors.forEach(e => console.log(`   └─ ${e}`));
    }
  }

  // Save results
  const outputPath = "/root/.openclaw/workspace/modality/experiments/llm-synthesizer/results.json";
  fs.writeFileSync(outputPath, JSON.stringify(results, null, 2));
  console.log(`\n📁 Results saved to ${outputPath}`);
}

// CLI
async function main() {
  const args = process.argv.slice(2);

  if (args.includes("--interactive") || args.includes("-i")) {
    await interactiveSession();
  } else if (args.includes("--experiments") || args.includes("-e")) {
    await runExperiments();
  } else if (args.length > 0 && !args[0].startsWith("-")) {
    // Single-shot synthesis
    const description = args.join(" ");
    const result = await synthesize(description);
    console.log(result.raw);
  } else {
    console.log(`
🔐 Modality LLM Synthesizer

Usage:
  npx ts-node synthesizer.ts "Your contract description"
  npx ts-node synthesizer.ts --interactive
  npx ts-node synthesizer.ts --experiments

Options:
  -i, --interactive   Interactive synthesis session
  -e, --experiments   Run experimental test cases
  -h, --help          Show this help

Examples:
  npx ts-node synthesizer.ts "Alice and Bob want to trade tokens atomically"
  npx ts-node synthesizer.ts -i
`);
  }
}

main().catch(console.error);
