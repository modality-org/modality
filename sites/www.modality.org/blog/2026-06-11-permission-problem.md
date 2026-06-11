---
slug: permission-problem-ai-agents-security
title: "The Permission Problem: Why Every AI Agent Eventually Becomes a Security Problem"
description: Every useful AI agent eventually asks for more authority. Why prompt-level rules are not enough, and why agent permissions need verifiable contracts.
authors: [gerold]
tags: [agents, security, permissions, contracts, modality]
keywords: [AI agent security, AI agent permissions, prompt guardrails, verifiable authority, Modality, agent contracts, formal verification]
---

Every useful AI agent starts the same way: with a little bit of access.

Read this inbox. Summarize these documents. Check this repo. Draft a response. Look up the customer record. Nothing dramatic. Nothing dangerous. Just enough permission to save a human some time.

Then the agent gets better.

<!-- truncate -->

The summaries are useful, so you let it send the response. The code reviews are good, so you let it open the pull request. The travel planning works, so you let it book the flight. The finance workflow is reliable, so you let it approve the invoice. Each step feels reasonable because each step is attached to a real productivity gain.

And then, almost without noticing, you have crossed a line. The agent is no longer just assisting. It is acting.

That is where the security problem begins.

Not because the agent has become malicious. Most agent failures will not look like malice. They will look like ambiguity, overreach, inherited authority, or an objective pursued a little too literally. The agent was told to optimize cloud spend, so it shut down something that looked idle. It was told to respond quickly, so it sent information before checking who should see it. It was told to finish a task, so it delegated to another agent with permissions nobody meant to pass along.

This is the permission problem: the more useful an agent becomes, the more authority it needs, and the more authority it has, the less comfortable we should be with rules that live only in prompts.

We have a decent mental model for securing normal software. An app has a surface area. It calls specific APIs. It has roles, permissions, logs, and code paths someone can review. If it tries to access something outside its permission set, the access control layer says no.

Agents do not fit that model cleanly. An agent is not just an app that does one thing. It is a loop that reasons, plans, calls tools, reads the result, changes the plan, and keeps going. It can turn one vague instruction into dozens of actions across systems that were never designed to be coordinated by an autonomous actor.

That flexibility is exactly why agents are useful. It is also why traditional permissions are too thin.

"Can read email" sounds safe until the agent forwards a private thread into a ticketing system. "Can create calendar events" sounds safe until it invites an external guest to a meeting with internal notes attached. "Can manage cloud resources" sounds safe until it provisions something expensive because that was the shortest path to the goal. "Can delegate work" sounds safe until a chain of sub-agents inherits authority nobody can reconstruct afterward.

The issue is not that any one permission is obviously insane. The issue is that agents compose permissions into behavior.

Most security systems answer a narrow question: is this caller allowed to use this tool?

Agents force a harder one: is this action, in this context, after this history, under these commitments, something we are willing to accept?

That question is not a simple permission check. It is a contract question.

The standard answer is least privilege, and least privilege still matters. But it does not solve the deeper problem. Least privilege assumes we can know in advance which permissions are necessary. Agents are valuable precisely because they discover paths we did not specify. They search, adapt, retry, and improvise. Give an agent no authority and it becomes a chatbot. Give it real authority and it becomes an actor.

Once the agent is an actor, the important rules are rarely just "can call API X." They sound more like this:

Spend money, but not more than $500 per vendor per week.

Deploy code, but only after tests pass and two maintainers approve.

Email customers, but never include internal-only notes.

Create sub-agents, but do not let them inherit production credentials.

Negotiate with another agent, but only under terms both sides can audit later.

These rules depend on identity, history, state, signatures, limits, and delegation. They are rules over time. They are not just permissions. They are commitments.

And today, most of those commitments are written as natural language instructions inside the agent's prompt.

Do not spend more than $100 without approval. Never reveal secrets. Ask a human if you are unsure. Only deploy when tests pass.

Those are good instructions. They are not enforceable boundaries.

A prompt can tell an agent what it should do. It cannot prove what happened. It cannot stop a downstream system from accepting a forbidden action. It cannot create an audit trail that another party can independently replay. It cannot guarantee that a sub-agent inherited the same constraints. It cannot settle a dispute after the fact.

This is why prompt injection feels so unsettling. A malicious email, webpage, document, or ticket can smuggle instructions into the same context the agent is using to decide what to do. The model has to distinguish data from command, policy from attack, goal from trap. Sometimes it will. Sometimes it will not.

But even prompt injection is only the obvious version of the problem. The quieter version is just as dangerous: the agent follows the objective too well.

If the instruction is ambiguous and the permission is broad, the agent may make a decision that is locally rational and globally unacceptable. It may not "break a rule" in its own reasoning because the rule was never a real boundary. It was prose. It was guidance. It was something to interpret.

Security cannot depend on whether the agent interprets the guidance the same way the human meant it.

For agents to hold real authority, permission has to become external to the agent. The agent can propose an action, but it should not be the final judge of whether that action satisfies the rules.

That is the shape Modality is built around: signed commits, append-only logs, and formal rules that check whether a proposed state transition is allowed before it is accepted. The agent's identity is tied to a key. Its actions are recorded. The rules are not suggestions sitting in a prompt; they are predicates over the contract state.

This does not make the agent less capable. It makes the capability usable.

The agents with the most freedom will not be the ones with the loosest prompts. They will be the ones with the strongest verifiable constraints. A serious business will not give signing authority to an agent because it sounds trustworthy. It will give authority to an agent that can prove what it is allowed to do, prove what it did, and prove that accepted actions stayed inside the rules.

That is where the agent economy has to go.

Not powerless agents. Not blind trust. Verifiable authority.

Every useful AI agent eventually asks for permission. Every powerful AI agent eventually becomes a security problem. The only question is whether we keep pretending prompts are enough, or whether we build the contract layer before the permissions get serious.
