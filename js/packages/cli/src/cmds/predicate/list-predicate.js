export async function listPredicates(argv) {
  const contractId = argv["contract-id"] || argv.contractId;

  console.log(`\n📋 Predicates in contract: ${contractId}\n`);

  if (contractId === "modal.money") {
    // Standard network predicates
    console.log("Standard Network Predicates:");
    console.log("━".repeat(80));
    
    const standardPredicates = [
      {
        name: "signed_by",
        path: "/_code/modal/signed_by.wasm",
        description: "Verify cryptographic signatures",
        args: "{ message, signature, public_key }",
        gasUsage: "100-200",
      },
      {
        name: "amount_in_range",
        path: "/_code/modal/amount_in_range.wasm",
        description: "Check numeric bounds",
        args: "{ amount, min, max }",
        gasUsage: "20-30",
      },
      {
        name: "has_property",
        path: "/_code/modal/has_property.wasm",
        description: "Check JSON property existence",
        args: "{ path, required }",
        gasUsage: "30-50",
      },
      {
        name: "timestamp_valid",
        path: "/_code/modal/timestamp_valid.wasm",
        description: "Validate timestamp constraints",
        args: "{ timestamp, max_age_seconds? }",
        gasUsage: "25-35",
      },
      {
        name: "post_to_path",
        path: "/_code/modal/post_to_path.wasm",
        description: "Verify commit actions",
        args: "{ path }",
        gasUsage: "40-100",
      },
    ];

    for (const pred of standardPredicates) {
      console.log(`\n  ${pred.name}`);
      console.log(`  ${"─".repeat(pred.name.length)}`);
      console.log(`  Path:        ${pred.path}`);
      console.log(`  Description: ${pred.description}`);
      console.log(`  Arguments:   ${pred.args}`);
      console.log(`  Gas Usage:   ${pred.gasUsage}`);
    }

    console.log("\n" + "━".repeat(80));
    console.log(`\nTotal: ${standardPredicates.length} predicates\n`);
    console.log("💡 Use 'modal predicate info <name>' for more details");
    console.log("💡 Use 'modal predicate test <name> --args <json>' to test\n");
  } else {
    console.log(`⚠️  Custom contract predicates listing not yet implemented`);
    console.log(`   This would query the datastore for /${contractId}/_code/*.wasm\n`);
  }
}

