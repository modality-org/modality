export async function predicateInfo(argv) {
  const name = argv.name;
  const contractId = argv["contract-id"] || argv.contractId || "modal.money";

  console.log(`\n📖 Predicate Information: ${name}\n`);
  console.log("━".repeat(80));

  const predicateInfo = {
    signed_by: {
      name: "signed_by",
      path: "/_code/modal/signed_by.wasm",
      description: "Verify cryptographic signatures using public key cryptography",
      arguments: {
        message: "string - The message that was signed",
        signature: "string - The signature to verify",
        public_key: "string - The public key to verify against",
      },
      returns: {
        valid: "boolean - true if signature is valid",
        gas_used: "number - Gas consumed (typically 100-200)",
        errors: "array - Error messages if validation fails",
      },
      examples: [
        {
          input: '{"message": "hello", "signature": "sig123", "public_key": "pk456"}',
          output: "+signed_by (if valid) or -signed_by (if invalid)",
        },
      ],
      gasUsage: "100-200 (crypto operations are expensive)",
      notes: [
        "Signature format depends on the cryptographic algorithm",
        "Public key must match the format expected by the algorithm",
        "Message is hashed before verification",
      ],
    },
    amount_in_range: {
      name: "amount_in_range",
      path: "/_code/modal/amount_in_range.wasm",
      description: "Check if a numeric value is within specified bounds",
      arguments: {
        amount: "number - The value to check",
        min: "number - Minimum allowed value (inclusive)",
        max: "number - Maximum allowed value (inclusive)",
      },
      returns: {
        valid: "boolean - true if amount >= min && amount <= max",
        gas_used: "number - Gas consumed (typically 20-30)",
        errors: "array - Error messages if validation fails",
      },
      examples: [
        {
          input: '{"amount": 100, "min": 0, "max": 1000}',
          output: "+amount_in_range (100 is in range [0, 1000])",
        },
        {
          input: '{"amount": 1500, "min": 0, "max": 1000}',
          output: "-amount_in_range (1500 exceeds maximum)",
        },
      ],
      gasUsage: "20-30 (simple arithmetic)",
      notes: [
        "Bounds are inclusive (min and max values are allowed)",
        "All values must be valid numbers",
        "Useful for transaction amounts, balances, rates",
      ],
    },
    has_property: {
      name: "has_property",
      path: "/_code/modal/has_property.wasm",
      description: "Check if a JSON object has a specific property",
      arguments: {
        path: "string - JSON path to check (dot notation)",
        required: "boolean - Whether the property must exist",
      },
      returns: {
        valid: "boolean - true if property exists (when required=true)",
        gas_used: "number - Gas consumed (typically 30-50)",
        errors: "array - Error messages if validation fails",
      },
      examples: [
        {
          input: '{"path": "user.email", "required": true}',
          output: "+has_property (if user.email exists)",
        },
      ],
      gasUsage: "30-50 (JSON traversal)",
      notes: [
        "Uses dot notation for nested properties (e.g., 'user.address.city')",
        "Useful for schema validation",
        "Can check for optional vs required fields",
      ],
    },
    timestamp_valid: {
      name: "timestamp_valid",
      path: "/_code/modal/timestamp_valid.wasm",
      description: "Validate timestamp against age constraints",
      arguments: {
        timestamp: "number - Unix timestamp to validate",
        max_age_seconds: "number - Maximum age in seconds (optional)",
      },
      returns: {
        valid: "boolean - true if timestamp is valid",
        gas_used: "number - Gas consumed (typically 25-35)",
        errors: "array - Error messages if validation fails",
      },
      examples: [
        {
          input: '{"timestamp": 1234567890, "max_age_seconds": 3600}',
          output: "+timestamp_valid (if within 1 hour)",
        },
      ],
      gasUsage: "25-35 (time comparison)",
      notes: [
        "Useful for expiry checks",
        "max_age_seconds is optional (checks format only if omitted)",
        "Compares against context timestamp",
      ],
    },
    post_to_path: {
      name: "post_to_path",
      path: "/_code/modal/post_to_path.wasm",
      description: "Verify that a commit includes a POST action to a specific path",
      arguments: {
        path: "string - The path to check for",
      },
      returns: {
        valid: "boolean - true if commit includes POST to path",
        gas_used: "number - Gas consumed (typically 40-100)",
        errors: "array - Error messages if validation fails",
      },
      examples: [
        {
          input: '{"path": "/_code/my_validator.wasm"}',
          output: "+post_to_path (if commit posts to that path)",
        },
      ],
      gasUsage: "40-100 (commit parsing)",
      notes: [
        "Useful for validating contract configuration",
        "Can verify WASM uploads",
        "Checks the current commit being processed",
      ],
    },
  };

  const info = predicateInfo[name];

  if (!info) {
    console.log(`\n❌ Unknown predicate: ${name}\n`);
    console.log(`Available predicates:`);
    console.log(`  - signed_by`);
    console.log(`  - amount_in_range`);
    console.log(`  - has_property`);
    console.log(`  - timestamp_valid`);
    console.log(`  - post_to_path\n`);
    return;
  }

  console.log(`\nName: ${info.name}`);
  console.log(`Path: ${info.path}`);
  console.log(`\nDescription:`);
  console.log(`  ${info.description}`);

  console.log(`\nArguments:`);
  for (const [key, desc] of Object.entries(info.arguments)) {
    console.log(`  ${key}: ${desc}`);
  }

  console.log(`\nReturns:`);
  for (const [key, desc] of Object.entries(info.returns)) {
    console.log(`  ${key}: ${desc}`);
  }

  console.log(`\nGas Usage: ${info.gasUsage}`);

  console.log(`\nExamples:`);
  for (const example of info.examples) {
    console.log(`  Input:  ${example.input}`);
    console.log(`  Output: ${example.output}`);
    console.log(``);
  }

  if (info.notes && info.notes.length > 0) {
    console.log(`Notes:`);
    for (const note of info.notes) {
      console.log(`  • ${note}`);
    }
  }

  console.log("\n" + "━".repeat(80));
  console.log(`\n💡 Test this predicate:`);
  console.log(`   modal predicate test ${name} --args '${info.examples[0].input}'\n`);
}

