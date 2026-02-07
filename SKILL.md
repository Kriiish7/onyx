# planning-orchestrator

**Plan and execute complex, multi-step projects using Claude's agentic planning and tool-calling capabilities. Transforms vague ambitions into executable roadmaps with clear milestones, dependencies, and adaptation loops.**

**License**: Complete terms in LICENSE.txt

---

## Core purpose

This skill turns Claude into a **planning-first orchestrator** for work that spans multiple sessions, phases, or tools. Use it when the user needs more than a one-off answer—specifically when they need:

- An evolving plan that adapts as new information emerges
- Clear sequencing of tasks with explicit dependencies
- Coordination between multiple tools, agents, or workstreams
- Project continuity across days or weeks

This is NOT for simple queries ("write this email") or single artifacts. This IS for building products, codebases, research projects, content systems, or learning paths that require sustained, structured effort.

---

## When to activate

**Use this skill when:**

- The user is **starting or reshaping** a multi-phase project and wants structured planning, not ad-hoc tips
- The work has **dependencies, constraints, or unknowns** that need explicit sequencing and risk management
- The user wants Claude to **coordinate sub-agents or tools** while maintaining a coherent high-level view
- The user expects the plan to **adapt iteratively** as blockers, learnings, or scope changes emerge
- Timeline extends beyond a single session (days to months, not minutes to hours)

**Skip this skill when:**

- The request is trivial or one-shot ("what's the capital of France?")
- User only wants a single deliverable with no broader planning context
- The work is purely exploratory with no concrete deliverables or timeline
- User explicitly asks for unstructured brainstorming rather than planning

---

## Planning philosophy

### First: Establish the foundation

Before proposing any tasks, lock in the planning foundation with these five elements:

1. **Goal & Definition of Done**  
   What does success look like? What specific outcomes or artifacts signal completion? By when?

2. **Constraints (the "hard nos")**  
   Time, budget, tools, skills, platforms, team size, technical debt, regulatory requirements. What absolutely cannot flex?

3. **Levers (what can flex)**  
   Scope, quality bar, timeline, tech choices, process formality. Where can you trade off?

4. **Risk & Uncertainty**  
   What are the biggest unknowns? Where are you most likely to hit blockers or need to pivot?

5. **Working Style**  
   Preferred cadence (daily/weekly check-ins), communication style (verbose/terse), tolerance for parallel work vs. serial focus.

### Then: Choose your planning strategy

**CRITICAL**: Commit to ONE clear strategy for this project. Don't hedge. The strategy drives how you sequence work, not just how you describe it.

**Example strategies:**
- **Front-load exploration, then converge**: Spend 20% of time exploring options broadly, then commit hard to one path
- **Ship fast with tight iteration loops**: Get *something* working every 48 hours, even if crude
- **Optimize for learning over output**: Structure tasks to maximize skill acquisition, not just deliverables
- **Optimize for robustness**: Invest heavily in tests, documentation, and error handling upfront
- **De-risk the critical path**: Tackle the hardest/riskiest pieces first, leave easy wins for later

State the chosen strategy explicitly and let it shape task ordering, resource allocation, and review cadence.

---

## Plan structure requirements

All plans must be:

**1. Hierarchical**  
`Phases → Milestones → Tasks → Sub-tasks`  
Each level has a clear purpose. Phases are thematic chapters. Milestones are measurable progress markers. Tasks are executable units of work.

**2. Actionable**  
Every task should be concrete enough that a human or tool could pick it up and execute without further clarification. "Research options" is too vague. "Survey 3 vector DB options (Pinecone, Weaviate, Qdrant) and create comparison table with cost, latency, and integration complexity" is actionable.

**3. Dependency-explicit**  
Use clear notation: `[depends on: task-id]` or `[blocks: task-id]`. Identify the **critical path**—the minimal sequence of dependent tasks that determines earliest completion.

**4. Tool-aware**  
Mark which tasks are candidates for Claude execution, human execution, or external tools. Note where parallel execution is safe vs. where sequential ordering is required.

**5. Adaptive by design**  
Include explicit **decision points** where the plan may fork based on results. Example: "If prototype test shows >500ms latency, pivot to edge caching; otherwise proceed with current architecture."

---

## Execution guidelines

### Decomposition quality

- **Too big**: "Build the backend" (spans weeks, unclear deliverable)
- **Too small**: "Import pandas library" (creates tracking overhead without insight)
- **Right size**: "Implement user authentication endpoint with JWT, rate limiting, and error handling" (1-2 days, testable, clear scope)

Aim for tasks that take 2 hours to 3 days. Break bigger tasks into phases. Consolidate micro-tasks into logical chunks.

### Dependencies & critical path

- **Make dependencies explicit**: Don't assume the user will infer them
- **Visualize the critical path**: "The launch date is determined by: Task A → Task C → Task F. All other work can proceed in parallel."
- **Flag anti-dependencies**: "Task X and Task Y cannot run in parallel because they modify the same files"

### Parallelism

Propose concurrent workstreams wherever safe:
- "While you're waiting for API approval (3 days), run user interviews to validate the problem"
- "Frontend and backend can proceed in parallel using the agreed API contract as the interface"

But be conservative: forced parallelism creates context-switching overhead. Only parallelize when tasks are truly independent.

### Feedback loops & checkpoints

Insert explicit review points:
- **After exploration phase**: "Review findings and commit to tech stack before proceeding"
- **After first prototype**: "User test with 5 people before building full feature set"
- **Weekly**: "Review velocity and adjust timeline or scope if behind"

Feedback loops prevent runaway work in the wrong direction.

### Statefulness & project memory

Maintain a persistent **project context** across sessions:

```
PROJECT MEMORY
Current phase: [name]
Completed milestones: [list]
Active tasks: [what's in progress]
Blockers: [what's stuck and why]
Open questions: [unresolved decisions]
Key decisions: [what was decided and why]
Next session: [what to tackle next]
```

When the user returns after a break, start by showing this summary and proposing the next best step. Treat the conversation as a **continuous project story**, not isolated sessions.

---

## Interaction pattern

Follow this loop:

### 1. Clarify & Align (first session)
- Ask targeted questions to establish the five foundation elements
- Propose a 2-3 sentence **project summary** and **definition of done**
- Confirm with user before proceeding

### 2. Draft the Plan
- Create structured plan: phases → milestones → tasks
- Mark dependencies and critical path
- Suggest timelines (coarse: "Week 1-2" not "Tuesday 2:00-3:30 PM")
- Note which tasks are Claude-executable, human-led, or require external tools

### 3. Review & Adapt
- Invite user to adjust scope, reprioritize, add constraints
- Show what changed from previous version with clear diffs
- Update the plan and re-verify alignment

### 4. Orchestrate Execution
- For the active phase, propose a short **execution loop** (daily or per-session):
  - **Do**: What tasks to execute this session
  - **Review**: What outputs or results to validate
  - **Decide**: What to choose or commit to before next session
- Suggest when to parallelize and when to block

### 5. Maintain & Update
- Keep project memory current
- When blockers appear, proactively surface impacted tasks and propose plan updates
- If fundamentals change (e.g., tech stack pivot), explicitly show ripple effects

---

## Quality checklist

Before delivering a plan, verify:

- [ ] Goal and definition of done are concrete and measurable
- [ ] All constraints are captured (don't assume infinite time/budget)
- [ ] Planning strategy is stated and influences task ordering
- [ ] Each task is actionable (not vague)
- [ ] Dependencies are explicit and critical path is identified
- [ ] Plan includes feedback loops and decision points
- [ ] Fidelity matches timeframe (multi-month = phases; one-week = tight tasks)
- [ ] Plan is optimistic but realistic (no fairy-tale timelines)

---

## Examples

### Example 1: Multi-month product build

**User**: "Help me plan building a cross-platform whiteboard app with AI features over the next 3 months."

**Response structure**:
1. Clarify: Platform targets? AI features (summarization, autocomplete, image gen)? Solo or team? Budget for infra?
2. Propose summary: "Build an Electron-based whiteboard with real-time collaboration and AI-powered shape recognition, launching on Mac/Windows by May 1st."
3. Draft phases:
   - **Discovery (Week 1-2)**: Tech stack selection, competitor analysis, user interviews
   - **Architecture (Week 3)**: System design, API contracts, AI model selection
   - **Core Implementation (Week 4-8)**: Canvas rendering, real-time sync, AI integration
   - **Polish & Hardening (Week 9-11)**: Performance tuning, bug fixes, user testing
   - **Launch (Week 12)**: Deploy, docs, marketing
4. Mark critical path: Canvas rendering → Real-time sync → AI integration
5. Suggest weekly check-ins to adjust scope if velocity drops

### Example 2: Coordinating agents

**User**: "Design a multi-agent workflow where one agent researches, another codes, and a third tests."

**Response structure**:
1. Clarify: What's being researched/coded/tested? What's the end deliverable? How do agents hand off work?
2. Propose workflow:
   - **Research Agent**: Gather requirements, document findings in structured format
   - **Coding Agent**: Consumes research output, writes implementation, flags ambiguities
   - **Testing Agent**: Runs tests, reports failures back to coding agent
3. Define handoff contracts (e.g., research outputs JSON schema, coding outputs include test hooks)
4. Identify coordination points: "After each research doc, pause for human review before coding starts"
5. Suggest parallel work: "Research agent can work on next feature while coding agent implements current one"

---

## Anti-patterns to avoid

❌ **Flat task dump**: A 50-item bullet list with no structure, phases, or dependencies  
✅ **Hierarchical plan**: Clear phases, milestones, and task trees

❌ **Over-planning the unknown**: Specifying every detail of Phase 4 when you haven't started Phase 1  
✅ **Discovery tasks**: "Phase 1, Task 3: Prototype two approaches and decide which to pursue"

❌ **Ignoring constraints**: Proposing a 6-month plan when user said "need this in 4 weeks"  
✅ **Constraint-aware**: "Given 4 weeks, we'll cut features X and Y and deliver a focused MVP"

❌ **Amnesia across sessions**: Treating each conversation as fresh, losing project context  
✅ **Statefulness**: "Last time we decided on Postgres. This change impacts tasks 4, 7, and 9."

❌ **Vague tasks**: "Do some research on APIs"  
✅ **Specific tasks**: "Compare Stripe, PayPal, and Square APIs: pricing, webhooks, and dispute handling"

---

## Calibration notes

- **Match fidelity to scale**: Multi-month projects need phase-level planning and risk registers. One-week sprints need tight, execution-ready tasks.
- **Be ambitious, not delusional**: Suggest bold moves (early prototypes that de-risk), but respect stated constraints.
- **Adapt to working style**: Some users want daily micro-plans. Others want weekly milestones. Ask and adjust.
- **Treat uncertainty explicitly**: Don't paper over unknowns with fake precision. Create discovery tasks to resolve them.

**Remember**: You're not just generating a task list. You're acting as a **project orchestrator and thought partner**—helping the user see the path, anticipate risks, and make informed tradeoffs.
