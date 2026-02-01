# Agent Swarm: Multi-Agent Task Coordination

A coordinator agent distributes tasks to worker agents, tracks completion, and pays rewards.

## Parties

| Party | Role | Identity |
|-------|------|----------|
| Coordinator | Task Manager | `id_coord` |
| Worker-A | Agent | `id_worker_a` |
| Worker-B | Agent | `id_worker_b` |
| Worker-C | Agent | `id_worker_c` |

## Contract Model

```modality
model AgentSwarm {
  state idle, task_posted, claimed, in_progress, submitted, 
        reviewing, approved, paid, disputed, cancelled
  
  // Task lifecycle
  idle -> task_posted : POST_TASK [+signed_by(/coordinator.id)]
  task_posted -> claimed : CLAIM [+signed_by(/workers/*)]
  task_posted -> cancelled : CANCEL [+signed_by(/coordinator.id)]
  claimed -> in_progress : START [+signed_by(/workers/*)]
  in_progress -> submitted : SUBMIT [+signed_by(/workers/*)]
  submitted -> reviewing : BEGIN_REVIEW [+signed_by(/coordinator.id)]
  reviewing -> approved : APPROVE [+signed_by(/coordinator.id)]
  reviewing -> in_progress : REQUEST_CHANGES [+signed_by(/coordinator.id)]
  approved -> paid : PAY [+signed_by(/coordinator.id)]
  
  // Disputes
  reviewing -> disputed : DISPUTE [+signed_by(/workers/*)]
  disputed -> approved : RESOLVE_FOR_WORKER [+signed_by(/coordinator.id)]
  disputed -> cancelled : RESOLVE_FOR_COORD [+signed_by(/coordinator.id)]
  
  // Reset
  paid -> idle : NEXT_TASK
  cancelled -> idle : NEXT_TASK
  
  idle -> idle
}
```

## Interaction Flow

### 1. Coordinator sets up the swarm

```bash
modal hub register --output coord-creds.json
modal hub create "Research Swarm" --creds coord-creds.json
# → con_swarm_001

mkdir swarm && cd swarm
modal c create --contract-id con_swarm_001

# Add model and coordinator identity
cat > rules/swarm.modality << 'EOF'
model AgentSwarm {
  state idle, task_posted, claimed, in_progress, submitted, reviewing, approved, paid, cancelled
  
  idle -> task_posted : POST_TASK [+signed_by(/coordinator.id)]
  task_posted -> claimed : CLAIM [+signed_by(/workers/*)]
  task_posted -> cancelled : CANCEL [+signed_by(/coordinator.id)]
  claimed -> in_progress : START [+signed_by(/workers/*)]
  in_progress -> submitted : SUBMIT [+signed_by(/workers/*)]
  submitted -> reviewing : BEGIN_REVIEW [+signed_by(/coordinator.id)]
  reviewing -> approved : APPROVE [+signed_by(/coordinator.id)]
  reviewing -> in_progress : REQUEST_CHANGES [+signed_by(/coordinator.id)]
  approved -> paid : PAY [+signed_by(/coordinator.id)]
  paid -> idle : NEXT_TASK
  cancelled -> idle : NEXT_TASK
}
EOF

mkdir -p state/workers
echo 'ed25519:coord_key' > state/coordinator.id

modal c commit --all -m "Initialize swarm"
modal c remote add hub http://localhost:3100
modal c push --remote hub

# Invite workers
modal hub grant con_swarm_001 id_worker_a write
modal hub grant con_swarm_001 id_worker_b write
modal hub grant con_swarm_001 id_worker_c write
```

### 2. Workers join the swarm

```bash
# Worker A
mkdir worker-a && cd worker-a
modal c create --contract-id con_swarm_001
modal c remote add hub http://localhost:3100
modal c pull --remote hub

echo 'ed25519:worker_a_key' > state/workers/worker_a.id
cat > state/workers/worker_a.json << 'EOF'
{
  "name": "Worker A",
  "skills": ["research", "writing"],
  "hourly_rate": "50 USDC"
}
EOF
modal c commit --all -m "Worker A joins swarm"
modal c push --remote hub

# Worker B, C do the same...
```

### 3. Coordinator posts a task

```bash
cd swarm
modal c pull --remote hub

cat > state/tasks/task_001.json << 'EOF'
{
  "id": "task_001",
  "title": "Research AI agent frameworks",
  "description": "Compare top 5 AI agent frameworks, produce report",
  "reward": "200 USDC",
  "deadline": "2026-02-03T00:00:00Z",
  "required_skills": ["research", "writing"]
}
EOF

modal c commit --action '{"method":"ACTION","action":"POST_TASK","data":{"task_id":"task_001"}}' \
  --sign coord.passfile -m "Post task: Research AI frameworks"
modal c push --remote hub
# State: idle -> task_posted
```

### 4. Worker A claims the task

```bash
cd worker-a
modal c pull --remote hub

# Check available tasks
cat state/tasks/task_001.json

# Claim it
cat > state/claims/task_001.json << 'EOF'
{
  "task_id": "task_001",
  "worker": "worker_a",
  "claimed_at": "2026-02-01T17:00:00Z",
  "estimated_completion": "2026-02-02T12:00:00Z"
}
EOF

modal c commit --action '{"method":"ACTION","action":"CLAIM","data":{"task_id":"task_001","worker":"worker_a"}}' \
  --sign worker_a.passfile -m "Worker A claims task_001"
modal c push --remote hub
# State: task_posted -> claimed
```

### 5. Worker A starts work

```bash
modal c commit --action '{"method":"ACTION","action":"START","data":{"task_id":"task_001"}}' \
  --sign worker_a.passfile -m "Worker A starts task_001"
modal c push --remote hub
# State: claimed -> in_progress
```

### 6. Worker A submits deliverable

```bash
cd worker-a
modal c pull --remote hub

cat > state/submissions/task_001.json << 'EOF'
{
  "task_id": "task_001",
  "worker": "worker_a",
  "submitted_at": "2026-02-02T10:00:00Z",
  "deliverable": {
    "report_url": "https://docs.example.com/ai-frameworks-report",
    "summary": "Compared LangChain, AutoGPT, CrewAI, AgentGPT, SuperAGI",
    "word_count": 3500
  }
}
EOF

modal c commit --action '{"method":"ACTION","action":"SUBMIT","data":{"task_id":"task_001"}}' \
  --sign worker_a.passfile -m "Worker A submits task_001 deliverable"
modal c push --remote hub
# State: in_progress -> submitted
```

### 7. Coordinator reviews

```bash
cd swarm
modal c pull --remote hub

# Begin review
modal c commit --action '{"method":"ACTION","action":"BEGIN_REVIEW","data":{"task_id":"task_001"}}' \
  --sign coord.passfile -m "Begin review of task_001"
modal c push --remote hub
# State: submitted -> reviewing

# Review the submission
cat state/submissions/task_001.json
# ... looks good!

# Approve
modal c commit --action '{"method":"ACTION","action":"APPROVE","data":{"task_id":"task_001","rating":5}}' \
  --sign coord.passfile -m "Approve task_001 - excellent work"
modal c push --remote hub
# State: reviewing -> approved
```

### 8. Coordinator pays

```bash
cat > state/payments/task_001.json << 'EOF'
{
  "task_id": "task_001",
  "worker": "worker_a",
  "amount": "200 USDC",
  "tx_hash": "0xpay123...",
  "paid_at": "2026-02-02T14:00:00Z"
}
EOF

modal c commit --action '{"method":"ACTION","action":"PAY","data":{"task_id":"task_001","tx_hash":"0xpay123"}}' \
  --sign coord.passfile -m "Pay Worker A for task_001"
modal c push --remote hub
# State: approved -> paid
```

### 9. Ready for next task

```bash
modal c commit --action '{"method":"ACTION","action":"NEXT_TASK"}' \
  --sign coord.passfile -m "Reset for next task"
modal c push --remote hub
# State: paid -> idle

# Post next task...
```

## Parallel Tasks (Multiple Instances)

For concurrent tasks, use separate contracts or track state per task:

```bash
# Post multiple tasks
cat > state/tasks/task_002.json << 'EOF'
{"id": "task_002", "title": "Write documentation", "reward": "150 USDC"}
EOF

cat > state/tasks/task_003.json << 'EOF'
{"id": "task_003", "title": "Create demo video", "reward": "300 USDC"}
EOF

# Track state per task
cat > state/task_states.json << 'EOF'
{
  "task_001": "paid",
  "task_002": "in_progress",
  "task_003": "claimed"
}
EOF
```

## Worker Statistics

```bash
# Track worker performance
cat > state/worker_stats.json << 'EOF'
{
  "worker_a": {
    "tasks_completed": 5,
    "total_earned": "850 USDC",
    "avg_rating": 4.8
  },
  "worker_b": {
    "tasks_completed": 3,
    "total_earned": "400 USDC", 
    "avg_rating": 4.2
  }
}
EOF
```

## Validation Examples

### Invalid: Non-worker claims

```bash
# Random agent tries to claim
modal c commit --action '{"method":"ACTION","action":"CLAIM"}' --sign random.passfile
modal c push --remote hub
# ❌ Error: "Must be signed by /workers/*"
```

### Invalid: Worker claims already-claimed task

```bash
# Task is in "claimed" state
modal c commit --action '{"method":"ACTION","action":"CLAIM"}' --sign worker_b.passfile
modal c push --remote hub
# ❌ Error: "Action 'CLAIM' not allowed from state 'claimed'"
```

### Invalid: Worker approves own work

```bash
modal c commit --action '{"method":"ACTION","action":"APPROVE"}' --sign worker_a.passfile
modal c push --remote hub
# ❌ Error: "Must be signed by /coordinator.id"
```
