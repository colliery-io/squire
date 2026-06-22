---
id: cash-out-as-a-redemption-replace
level: task
title: "Cash-out as a redemption (replace the per-squire Pay button)"
short_code: "SQUIRE-T-0118"
created_at: 2026-06-22T14:40:38.908544+00:00
updated_at: 2026-06-22T14:40:38.908544+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Cash-out as a redemption

## Why
The per-squire "Pay $X" button (SQUIRE-T-0111) made the Knight squire cards uneven (a variable extra
button) and conceptually it's the wrong home: paying out owed cash is a **balance draw-down with
parent approval** — i.e. a redemption. Operator-confirmed model (2026-06-22). The Pay button has been
**removed** from the Android Knight squire card; the Keep desktop adjust (currency=Cash) still covers
payout in the interim, so nothing is stranded.

## Model (confirmed)
Owed cash is redeemable. When a squire is owed $X:
1. The **child's Rewards** shows a **"Cash out $X"** entry next to the coin rewards.
2. Child taps it → a **redemption request** (same as redeeming a reward).
3. Parent sees it in the **Review queue** (pending redemptions) → **approve** pays it out (owed → $0),
   **reject** leaves it owed.

Child-requested, parent-approved, draws down the cash balance — the existing redemption flow, but for
the **Cash** currency instead of spending coins.

## Scope
- **Domain/server:** a cash redemption — a built-in/virtual "Cash out" that redeems the *cash* balance
  (vs. coin rewards that cost coins). Request → `RedemptionRequested`-style event; approval emits
  `Adjusted{Cash, −X}` and routes through the existing review/approve machinery. Decide: reserved
  built-in item vs. a dedicated `CashOut` command; affordability = owed-cash ≥ amount.
- **Child app:** surface "Cash out $X" in the Rewards tab when owed cash > 0 → request redemption.
- **Parent app:** the cash-out appears in the Review queue's pending redemptions (already renders
  redemptions — just label/format as cash).
- **Tests:** Gherkin — "a squire cashes out → parent approves → owed cash is 0" (api cucumber), plus
  the child/parent UI bits.
- **Cleanup:** remove the now-dead `onPay`/`PayDialog`/`KnightStore.pay` once this lands (or repurpose).

## Acceptance
- [ ] A squire owed cash can request a cash-out; the parent approves it in the review queue; owed → 0
  with an `Adjusted{Cash,−}` event; reject leaves it owed.
- [ ] No per-squire Pay button anywhere; cards stay uniform.
- [ ] Gherkin scenario green.

## Note
Deferred behind shipping the home redesign (tabs + first-name headers + uniform cards). Build as its
own focused piece with proper domain design + tests.

# Cash-out as a redemption (replace the per-squire Pay button)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[SQUIRE-I-0001]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*