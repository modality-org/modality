Create a Lean4 project in this directory.

This Lean4 project will attempt to replicate the results of Modality's Multi-Agent Chess experiment from scratch using Lean4 and modal contracts written in a Lean4 version of LTL.


## Modal Contracts

* Implement the Modal Contract concept: a contract is an append-only log of multi-actions with a LTS model that satisfies a set of temporal logic rules, each anchored at the commit that added the rule. When a rule is added an LTS model satisfying the full ruleset over the full history must be used.

* Implement a Lean4 version of LTL for the rules to be written in. If a suitable LTL library exists, use it.

* The Modal Contract maintains a state of Lean typed variables, the value of which is accumulated from the actions in the contract.

* The rules of the Modal Contract can use predicate functions to evaluate the state of the variables. Each predicate function returns True, False, or Unknown.

* The LTS transitions labels are a set of maps of predicate functions.

* When a model is checked Lean is used to ensure that the predicate functions do not contradict each other.

* Custom predicate functions are written in Lean4.

* When a modal contract is appended to with a commit, it must use an accessible transition that matches its content.


## Multi-Agent Demo

* Implement a simple example of modal contract, where Alice and Bob start off with no rules and then agree that 1) commits must be signed by either party and 2) commits with new rules must be signed by both parties. In this example have Alice and Bob provide their own satisfying models. Have Alice provide a model that satisfies the rules, but only has non-rule transitions that are signed by her. Have Bob replace the model with one that also works for him.

## Model synthesis

* Inspired by the model synthesis code from Modality, create a model synthesis tool for this experiment.

## Multi-Agent Chess

* Implement a simple example of multi-agent chess, just like the Modality experiment.