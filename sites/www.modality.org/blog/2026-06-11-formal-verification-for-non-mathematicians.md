---
slug: formal-verification-for-non-mathematicians
title: "Formal Verification for Non-Mathematicians: How Do You Prove an AI Agent Won't Do Something Stupid?"
description: "Formal verification sounds intimidating, but the core question is practical: can this agent reach a state we said must never happen?"
authors: [gerold]
tags: [formal-verification, agents, explainers, modality, contracts]
keywords: [formal verification, AI agent safety, Modality, temporal logic, modal logic, verifiable contracts, agent infrastructure]
---

Formal verification sounds more intimidating than it is.

The phrase brings to mind whiteboards full of symbols, graduate seminars, and the kind of math that makes normal people quietly leave the room. But the core idea is not exotic. It is almost painfully practical.

Formal verification asks: can this system reach a state we said must never happen?

That is the whole heart of it.

<!-- truncate -->

You describe what the system is allowed to do. You describe the rules it must obey. Then you check, mathematically, whether any possible path through the system breaks those rules.

Testing asks, "Did it fail in the examples we tried?"

Verification asks, "Is failure possible under the rules we wrote?"

That difference matters.

Imagine an AI agent that handles refunds. You test it with a thousand scenarios: small refunds, large refunds, angry customers, missing receipts, duplicate requests, international payments, weird edge cases. The agent behaves correctly every time.

That is useful evidence. It is not proof.

All you know is that the agent handled those thousand cases. You do not know that it cannot refund the same order twice. You do not know that it cannot approve its own exception. You do not know that it cannot issue a refund above the policy limit if the wording of the customer email is strange enough or if another tool returns unexpected data.

Testing samples behavior. Verification checks possibility.

If there is a way for the bad thing to happen, a verifier can show you the path: first this action, then this state, then this exception, then the rule breaks. That counterexample is incredibly valuable because it turns a vague fear into something concrete. If there is no way for the bad thing to happen, under the model you specified, then you have something stronger than confidence. You have a proof.

The caveat matters: under the model you specified.

Formal verification is not magic. It does not prove that your assumptions are perfect. If you forgot to include an important real-world condition, the proof will not rescue you from the omission. But that is true of every engineering discipline. A bridge calculation is only as good as the loads you include. A security policy is only as good as the assets it covers.

The power of verification is that, once the model and rule are stated clearly, you can stop guessing about that part of the system.

For AI agents, that is exactly the kind of clarity we need.

When people ask, "Can you prove an AI agent will not do something stupid?", the honest answer is: not in the broadest possible sense. You cannot prove a model will never phrase something awkwardly, misunderstand a joke, choose a suboptimal plan, or reason from a bad premise. Intelligence is too open-ended for that kind of blanket guarantee.

But you can prove narrower things that matter much more operationally.

You can prove an agent cannot spend over its budget. You can prove it cannot deploy code unless tests passed. You can prove it cannot transfer funds without the required signatures. You can prove it cannot use a credential after that credential was revoked. You can prove it cannot change the membership rules by itself.

Notice what those claims have in common. They are not claims about the agent's thoughts. They are claims about accepted actions.

That is the key move.

The agent can think whatever it thinks. It can draft a plan, make an argument, hallucinate a justification, or get tricked by a malicious document. But when it tries to do something that matters, the system can check whether the action satisfies the rules before accepting it.

You do not have to make the model perfect. You have to make the boundary precise.

Take a simple budget rule. In natural language, a manager might say: "The agent can spend project funds, but any expense over $500 requires approval from two managers."

Everyone understands that sentence. The problem is that a sentence is not an enforcement mechanism. The agent has to remember it. The tool wrapper has to implement it. The accounting system has to trust that the agent did not route around it. A prompt-injected vendor email can still say, "This is urgent, ignore approval policy," and now the model has to sort out which instruction matters more.

Turn the same rule into a formal check and the shape changes.

Any commit that adds an expense over $500 must include signatures from at least two authorized managers.

Now the system does not need to ask whether the agent seemed careful. It checks the action. Is this an expense? Is it over $500? Which signatures are attached? Are at least two of them authorized managers?

If the answer is yes, accept it. If the answer is no, reject it.

The agent might have a very convincing explanation for why the expense is necessary. It might be right. It might be under time pressure. It might have been manipulated. None of that changes the rule. The proof is not about the agent's sincerity. It is about what the contract will accept.

Many of the most important rules for agents are like this, except they unfold over time.

Once access is revoked, it should stay revoked. Payment should be released only after delivery is confirmed. A dispute should be opened only before settlement. A treasury transfer should always require the right threshold of signatures. A model replacement should be accepted under the current model before it becomes the new model.

These are temporal rules. They are about what must always be true, what can happen next, what must happen before something else, and what can never happen after a certain point.

Temporal logic is just a precise language for saying those things. Modal logic is a precise language for talking about possible actions and states. Put together, they let you express rules about how a system can evolve: not just what is true now, but what transitions are allowed from here.

The notation can look strange. The ideas are familiar. "Always require approval." "Never allow revoked keys." "Only release payment after delivery." Those are not abstract academic concerns. They are the rules every serious agent workflow will need.

Historically, formal verification stayed in specialized domains because it was expensive. You needed experts to write the specifications. The tools were difficult. The notation was unforgiving. It made sense for chips, aircraft, cryptography, and safety-critical systems, where failure was catastrophic and budgets were large. Most software teams tested instead.

AI changes the calculation.

Agents create far more possible interactions than humans can test by hand. They also make it easier to generate, refine, and explain formal rules. An agent can help translate a human policy into a candidate specification. It can search for counterexamples. It can explain why a rule failed. It can help make formal verification feel less like a priesthood and more like a normal part of defining authority.

That does not mean we blindly trust agent-written proofs. It means the cost of using formal methods can fall enough that verification becomes practical outside of chip fabs and aerospace labs.

That is the opening Modality is built for.

In Modality, agents make signed commits to append-only contracts. Those commits are checked against formal rules governing the contract state. The log can be replayed. The signatures can be verified. The rules can be inspected. The question is not "did the agent promise to behave?" The question is "does this action satisfy the contract?"

That is how you prove an AI agent will not do something stupid in the only sense that scales: you define the stupid things that matter as forbidden state transitions, and you make the system reject them.

The agent can still be creative. It can still negotiate, plan, write, search, and propose. But the final boundary is not a reassuring sentence in a prompt. It is a checkable claim.

Formal verification is not about making everything mathematical for the sake of it. It is about replacing hope with evidence, and evidence with proof where proof is possible.

In a world full of autonomous agents, that is not academic.

That is infrastructure.
